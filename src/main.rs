mod card_art;
mod codenames_view;
mod fih_art;
mod fih_character;
mod fih_view;
mod game_view;
mod ito_view;
mod local_pair_view;
mod mines_view;
mod nonograms_view;
mod online_net;
mod online_style;
mod online_view;
mod platform;
mod sudoku_view;
mod table_tennis_view;
mod telephone_view;
mod ui;
mod wavelength_view;
mod wolves_art;
mod wolves_view;

use fih_view::{FihPage, FihPreview, Nav};
use game_view::{Preview, draw_board};
use jarcade::{
    feedback::{Feedback, Pulse},
    fps::FpsCounter,
    layout::Layout,
    local_pair::Kind as PairKind,
    navigation::Route,
    settings::Settings,
    snake::{Cell, Direction, Snake, Status},
    snake_input::{SnakeInput, controls},
    snake_motion::{body_path, head_heading},
    timing::{Advance, TickClock},
};
use macroquad::prelude::*;
use mines_view::{MinesPage, MinesPreview};
use ui::{CORAL, GREEN, Icon, Ui, bordered, draw_icon, logo, rounded};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Settings,
    Game,
    Mines,
    Fih,
    Multiplayer,
    Wavelength,
    Ito,
    LocalPair(PairKind),
    Tennis,
    Nonograms,
    Sudoku,
}
#[derive(Clone, Copy)]
enum Action {
    None,
    Menu,
    CloseMenu,
    Navigate(Route),
    Home,
    Settings,
    NewGame,
    NewMines,
    NewFih,
    NewCourt,
    NewReverie,
    NewWolves,
    NewCodenames,
    NewTelephone,
    NewWavelength,
    NewIto,
    NewPair(PairKind),
    NewTennis,
    NewNonograms,
    NewSudoku,
    TogglePause,
    TogglePower,
    ToggleHaptics,
    ToggleFps,
}

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Jarcade".into(),
            window_width: 1000,
            window_height: 820,
            high_dpi: true,
            sample_count: 1,
            platform: miniquad::conf::Platform {
                blocking_event_loop: true,
                swap_interval: Some(1),
                ..Default::default()
            },
            ..Default::default()
        },
        update_on: Some(macroquad::conf::UpdateTrigger {
            key_down: true,
            mouse_down: true,
            mouse_up: true,
            mouse_wheel: true,
            touch: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

struct App {
    screen: Screen,
    drawer: bool,
    settings: Settings,
    game: Snake,
    clock: TickClock,
    feedback: Feedback,
    preview: Preview,
    mines: MinesPage,
    mines_preview: MinesPreview,
    fps: FpsCounter,
    fih: FihPage,
    fih_preview: FihPreview,
    online: online_view::OnlinePage,
    wavelength: wavelength_view::WavelengthPage,
    ito: ito_view::ItoPage,
    pair: local_pair_view::PairPage,
    tennis: table_tennis_view::TennisPage,
    nonograms: nonograms_view::NonogramsPage,
    sudoku: sudoku_view::SudokuPage,
    home_pan: jarcade::board_pan::BoardPan,
    home_gesture_blocked: bool,
    previous_body: Vec<Cell>,
    render_body: Vec<Vec2>,
    multiplayer: bool,
    save_failed: bool,
    haptics_supported: bool,
}

impl App {
    fn new(ui: &Ui) -> Self {
        let settings = platform::load_settings();
        platform::appearance(settings.power_saver, false);
        let fih = FihPage::new();
        let fih_preview = FihPreview::new(&fih.pet, settings.power_saver);
        Self {
            screen: Screen::Home,
            drawer: false,
            online: online_view::OnlinePage::new(),
            wavelength: wavelength_view::WavelengthPage::new(seed()),
            ito: ito_view::ItoPage::new(seed()),
            pair: local_pair_view::PairPage::new(),
            tennis: table_tennis_view::TennisPage::new(seed()),
            nonograms: nonograms_view::NonogramsPage::new(platform::load_nonograms()),
            sudoku: sudoku_view::SudokuPage::new(platform::load_sudoku()),
            home_pan: jarcade::board_pan::BoardPan::default(),
            home_gesture_blocked: false,
            settings,
            game: Snake::new(seed()),
            clock: TickClock::default(),
            feedback: Feedback::default(),
            preview: Preview::new(settings.power_saver),
            mines: MinesPage::new(seed()),
            mines_preview: MinesPreview::new(ui, settings.power_saver),
            fps: FpsCounter::default(),
            fih,
            fih_preview,
            previous_body: Vec::new(),
            render_body: Vec::new(),
            multiplayer: false,
            save_failed: false,
            haptics_supported: platform::haptics_supported(),
        }
    }
    fn route(&self) -> Route {
        use jarcade::{fih::Room, multiplayer::GameKind};
        match self.screen {
            Screen::Home => {
                if self.multiplayer {
                    Route::Multiplayer
                } else {
                    Route::SinglePlayer
                }
            }
            Screen::Settings => Route::Settings,
            Screen::Game => Route::Snake,
            Screen::Mines => {
                if self.mines.configuring {
                    Route::Mines
                } else {
                    Route::MinesPlay
                }
            }
            Screen::Fih => match self.fih.room() {
                Room::Kitchen => Route::Kitchen,
                Room::Bathroom => Route::Bathroom,
                Room::Bedroom => Route::Bedroom,
                Room::Playroom => Route::Playroom,
                Room::Clinic => Route::Clinic,
            },
            Screen::Multiplayer => match self.online.game {
                GameKind::Court => Route::Coupe,
                GameKind::Reverie => Route::Dicksit,
                GameKind::Wolves => Route::Wolvesville,
                GameKind::Codenames => Route::Codenames,
                GameKind::Telephone => Route::DrawingTelephone,
            },
            Screen::Wavelength => Route::Wavelength,
            Screen::Ito => Route::Ito,
            Screen::LocalPair(kind) => match kind {
                PairKind::Faces => Route::GuessWho,
                PairKind::Mastermind => Route::Mastermind,
                PairKind::TicTacToe => Route::TicTacToe,
                PairKind::ConnectFour => Route::ConnectFour,
            },
            Screen::Tennis => Route::TableTennis,
            Screen::Nonograms => {
                if self.nonograms.playing() {
                    Route::NonogramsPlay
                } else {
                    Route::Nonograms
                }
            }
            Screen::Sudoku => {
                if self.sudoku.playing() {
                    Route::SudokuPlay
                } else {
                    Route::Sudoku
                }
            }
        }
    }
    fn interrupt(&mut self) {
        self.game.pause();
        self.tennis.interrupt();
        self.fih.interrupt();
        self.mines.cancel_gesture();
        self.nonograms.cancel_gesture();
        self.sudoku.cancel_gesture();
        self.online.cancel_gesture();
        self.home_pan.cancel();
        if self.screen == Screen::Wavelength {
            self.wavelength.interrupt();
        }
        if self.screen == Screen::Ito {
            self.ito.interrupt();
        }
        if matches!(self.screen, Screen::LocalPair(_)) {
            self.pair.interrupt();
        }
        platform::editor_close();
    }
    fn set_drawer(&mut self, open: bool, ui: &mut Ui) {
        if open == self.drawer {
            return;
        }
        if open {
            self.interrupt();
        }
        self.drawer = open;
        ui.reset_focus();
    }
    /// History restores the existing session, rather than starting a new game.
    fn navigate(&mut self, route: Route, ui: &mut Ui, now: f64) {
        use jarcade::{fih::Room, multiplayer::GameKind};
        let previous = self.screen;
        self.interrupt();
        match route {
            Route::SinglePlayer | Route::Multiplayer => {
                if previous == Screen::Fih {
                    self.fih_preview = FihPreview::new(&self.fih.pet, self.settings.power_saver);
                }
                let multiplayer = route == Route::Multiplayer;
                if self.multiplayer != multiplayer {
                    self.home_pan = jarcade::board_pan::BoardPan::default();
                }
                self.screen = Screen::Home;
                self.multiplayer = multiplayer;
            }
            Route::Settings => self.screen = Screen::Settings,
            Route::Snake => self.screen = Screen::Game,
            Route::Mines | Route::MinesPlay => {
                self.screen = Screen::Mines;
                self.mines.configuring = route == Route::Mines;
            }
            Route::Nonograms | Route::NonogramsPlay => {
                self.screen = Screen::Nonograms;
                self.nonograms.navigate(route == Route::NonogramsPlay);
            }
            Route::Sudoku | Route::SudokuPlay => {
                self.screen = Screen::Sudoku;
                self.sudoku.navigate(route == Route::SudokuPlay);
            }
            Route::Fih
            | Route::Kitchen
            | Route::Bathroom
            | Route::Bedroom
            | Route::Playroom
            | Route::Clinic => {
                self.screen = Screen::Fih;
                if previous != Screen::Fih {
                    self.fih.enter(now);
                }
                let room = match route {
                    Route::Bathroom => Room::Bathroom,
                    Route::Bedroom => Room::Bedroom,
                    Route::Playroom => Room::Playroom,
                    Route::Clinic => Room::Clinic,
                    Route::Kitchen => Room::Kitchen,
                    _ => self.fih.room(),
                };
                self.fih.move_room(room, ui);
            }
            Route::GuessWho | Route::Mastermind | Route::TicTacToe | Route::ConnectFour => {
                let kind = match route {
                    Route::GuessWho => PairKind::Faces,
                    Route::Mastermind => PairKind::Mastermind,
                    Route::TicTacToe => PairKind::TicTacToe,
                    _ => PairKind::ConnectFour,
                };
                self.pair.enter(kind);
                self.screen = Screen::LocalPair(kind);
                self.multiplayer = true;
            }
            Route::Ito => {
                self.screen = Screen::Ito;
                if previous != self.screen {
                    self.ito.enter();
                }
                self.multiplayer = true;
            }
            Route::Wavelength => {
                self.screen = Screen::Wavelength;
                if previous != self.screen {
                    self.wavelength.enter();
                }
                self.multiplayer = true;
            }
            Route::TableTennis => self.screen = Screen::Tennis,
            Route::Coupe
            | Route::Dicksit
            | Route::Wolvesville
            | Route::Codenames
            | Route::DrawingTelephone => {
                let kind = match route {
                    Route::Dicksit => GameKind::Reverie,
                    Route::Wolvesville => GameKind::Wolves,
                    Route::Codenames => GameKind::Codenames,
                    Route::DrawingTelephone => GameKind::Telephone,
                    _ => GameKind::Court,
                };
                if previous != Screen::Multiplayer || self.online.game != kind {
                    let code = platform::invite()
                        .filter(|(game, _)| *game == kind)
                        .map(|(_, code)| code);
                    self.online.enter(kind, code);
                }
                self.screen = Screen::Multiplayer;
                self.multiplayer = true;
            }
        }
        if previous == Screen::Multiplayer && self.screen != previous {
            self.online.suspend();
        }
        ui.reset_focus();
        platform::appearance(self.settings.power_saver, self.screen == Screen::Wavelength);
    }
    fn sidebar(&mut self, ui: &mut Ui) -> Action {
        let width = (screen_width() * 0.86).min(320.);
        // Keep hidden game state private and avoid GPU readbacks or idle snapshots.
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0., 0., 0., 0.16),
        );
        draw_rectangle(0., 0., width, screen_height(), ui.theme.bg);
        logo(Rect::new(20., 22., 26., 26.), ui.theme.accent);
        ui.heading("jarcade", 56., 44., 22., ui.theme.text);
        if ui.icon_button(Icon::Back, Rect::new(width - 56., 12., 44., 44.), false) {
            return Action::CloseMenu;
        }
        let route = self.route();
        let mut y = 76.;
        if !route.is_menu() {
            ui.label("IN PLAY", 20., y + 12., 11., ui.theme.muted);
            y += 24.;
            if ui.button(
                &format!("Continue {}", route.title()),
                Rect::new(16., y, width - 32., 48.),
                true,
            ) {
                return Action::CloseMenu;
            }
            y += 68.;
        }
        for destination in [Route::SinglePlayer, Route::Multiplayer] {
            if ui.button(
                destination.title(),
                Rect::new(16., y, width - 32., 48.),
                route == destination,
            ) {
                return Action::Navigate(destination);
            }
            y += 56.;
        }
        // App configuration belongs to the launch screen, including this drawer.
        if route.is_menu()
            && ui.button(
                "Settings",
                Rect::new(16., y, width - 32., 48.),
                route == Route::Settings,
            )
        {
            return Action::Navigate(Route::Settings);
        }
        if ui.hit(Rect::new(
            width,
            0.,
            screen_width() - width,
            screen_height(),
        )) {
            return Action::CloseMenu;
        }
        Action::None
    }
    fn save(&mut self) {
        self.save_failed = !platform::save_settings(self.settings);
    }
    fn pulse(&mut self, pulse: Pulse, now: f64) {
        if let Some(pulse) =
            self.feedback
                .request(pulse, self.settings.haptics && self.haptics_supported, now)
        {
            platform::haptic(pulse);
        }
    }
    fn pause_or_resume(&mut self, now: f64) {
        match self.game.status() {
            Status::Running => self.game.pause(),
            Status::Ready => {
                self.game.start_or_resume();
                self.clock.start(now);
            }
            Status::Paused => {
                self.game.start_or_resume();
                self.clock.resume(now);
            }
            _ => {}
        }
    }
    fn act(&mut self, action: Action, ui: &mut Ui, now: f64) -> bool {
        match action {
            Action::None => return false,
            Action::Menu | Action::CloseMenu => {
                let open = matches!(action, Action::Menu);
                platform::route_drawer(open);
                if !cfg!(target_arch = "wasm32") {
                    self.set_drawer(open, ui);
                }
            }
            Action::Navigate(route) => {
                if route == self.route() {
                    platform::route_drawer(false);
                    if !cfg!(target_arch = "wasm32") {
                        self.set_drawer(false, ui);
                    }
                } else {
                    self.set_drawer(false, ui);
                    self.navigate(route, ui, now);
                }
            }
            Action::Home => {
                self.tennis.interrupt();
                self.online.suspend();
                if matches!(self.screen, Screen::LocalPair(_)) {
                    self.pair.interrupt();
                }
                if self.screen == Screen::Wavelength {
                    self.wavelength.interrupt();
                }
                if self.screen == Screen::Ito {
                    self.ito.interrupt();
                }
                self.fih.interrupt();
                self.fih_preview = FihPreview::new(&self.fih.pet, self.settings.power_saver);
                self.mines.cancel_gesture();
                self.nonograms.cancel_gesture();
                self.sudoku.cancel_gesture();
                self.game.pause();
                self.screen = Screen::Home;
            }
            Action::Settings => {
                self.tennis.interrupt();
                self.fih.interrupt();
                self.game.pause();
                self.screen = Screen::Settings;
            }
            Action::NewGame => {
                self.game = Snake::new(seed());
                self.previous_body.clear();
                self.screen = Screen::Game;
                self.clock.reset(now);
            }
            Action::NewMines => {
                self.game.pause();
                self.mines.choose_size();
                self.screen = Screen::Mines;
            }
            Action::NewCourt
            | Action::NewReverie
            | Action::NewWolves
            | Action::NewCodenames
            | Action::NewTelephone => {
                self.game.pause();
                self.online.enter(
                    if matches!(action, Action::NewCourt) {
                        jarcade::multiplayer::GameKind::Court
                    } else if matches!(action, Action::NewReverie) {
                        jarcade::multiplayer::GameKind::Reverie
                    } else if matches!(action, Action::NewWolves) {
                        jarcade::multiplayer::GameKind::Wolves
                    } else if matches!(action, Action::NewTelephone) {
                        jarcade::multiplayer::GameKind::Telephone
                    } else {
                        jarcade::multiplayer::GameKind::Codenames
                    },
                    None,
                );
                self.screen = Screen::Multiplayer;
            }
            Action::NewPair(kind) => {
                self.game.pause();
                self.online.suspend();
                self.pair.enter(kind);
                self.screen = Screen::LocalPair(kind);
                self.multiplayer = true;
            }
            Action::NewIto => {
                self.game.pause();
                self.online.suspend();
                self.ito.enter();
                self.screen = Screen::Ito;
                self.multiplayer = true;
            }
            Action::NewWavelength => {
                self.game.pause();
                self.online.suspend();
                self.mines.cancel_gesture();
                self.wavelength.enter();
                self.screen = Screen::Wavelength;
            }
            Action::NewTennis => {
                self.game.pause();
                self.online.suspend();
                self.mines.cancel_gesture();
                self.tennis.enter(seed());
                self.screen = Screen::Tennis;
            }
            Action::NewNonograms => {
                self.game.pause();
                self.online.suspend();
                self.mines.cancel_gesture();
                self.nonograms.enter();
                self.screen = Screen::Nonograms;
            }
            Action::NewSudoku => {
                self.game.pause();
                self.online.suspend();
                self.sudoku.enter();
                self.screen = Screen::Sudoku;
            }
            Action::NewFih => {
                self.game.pause();
                self.mines.cancel_gesture();
                self.fih.enter(now);
                self.screen = Screen::Fih;
            }
            Action::ToggleFps => {
                self.settings.show_fps = !self.settings.show_fps;
                self.fps = FpsCounter::default();
                self.save();
            }
            Action::TogglePause => self.pause_or_resume(now),
            Action::TogglePower => {
                self.settings.power_saver = !self.settings.power_saver;
                self.preview = Preview::new(self.settings.power_saver);
                self.fih_preview = FihPreview::new(&self.fih.pet, self.settings.power_saver);
                self.mines_preview = MinesPreview::new(ui, self.settings.power_saver);
                self.save();
            }
            Action::ToggleHaptics => {
                self.settings.haptics = !self.settings.haptics;
                self.save();
            }
        }
        if matches!(
            action,
            Action::Home
                | Action::Settings
                | Action::NewGame
                | Action::NewMines
                | Action::NewFih
                | Action::NewCourt
                | Action::NewReverie
                | Action::NewWolves
                | Action::NewCodenames
                | Action::NewTelephone
                | Action::NewWavelength
                | Action::NewIto
                | Action::NewPair(_)
                | Action::NewTennis
                | Action::NewNonograms
                | Action::NewSudoku
        ) {
            self.home_pan.cancel();
            self.nonograms.cancel_gesture();
            self.sudoku.cancel_gesture();
            ui.reset_focus();
        }
        platform::appearance(self.settings.power_saver, self.screen == Screen::Wavelength);
        true
    }
    fn steer(&mut self, direction: Direction, now: f64) {
        let accepted = self.game.turn(direction);
        if self.game.status() == Status::Ready {
            self.game.start_or_resume();
            self.clock.start(now);
        }
        if accepted {
            self.pulse(Pulse::Tap, now);
        }
    }
    fn game_input(&mut self, ui: &Ui, input: &mut SnakeInput, now: f64) {
        if self.screen != Screen::Game {
            return;
        }
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::P) {
            self.pause_or_resume(now);
            self.pulse(Pulse::Tap, now);
        }
        for direction in input.turns.drain(..) {
            self.steer(direction, now);
        }
        if !ui.keyboard_focus && is_key_pressed(KeyCode::Enter) {
            if matches!(self.game.status(), Status::Lost | Status::Won) {
                self.game = Snake::new(seed());
                self.previous_body.clear();
            }
            if matches!(self.game.status(), Status::Ready | Status::Paused) {
                self.pause_or_resume(now);
            }
        }
    }

    fn game_controls(&mut self, ui: &mut Ui, layout: &Layout, now: f64) {
        for (direction, rect) in controls(layout.controls) {
            // Pointer presses already came through the ordered input stream.
            // Keep Tab/Enter activation here, before advancing the game too.
            if ui.icon_button(Icon::Arrow(direction.delta()), rect, false) && ui.keyboard_focus {
                self.steer(direction, now);
            }
        }
    }
    fn advance(&mut self, now: f64) {
        if self.screen != Screen::Game || self.game.status() != Status::Running {
            return;
        }
        match self.clock.advance(now) {
            Advance::Steps(steps) => {
                for _ in 0..steps {
                    if self.game.status() != Status::Running {
                        break;
                    }
                    if !self.settings.power_saver {
                        self.previous_body.clear();
                        self.previous_body.extend(self.game.body());
                    }
                    let old_score = self.game.score();
                    self.game.tick();
                    match self.game.status() {
                        Status::Lost => self.pulse(Pulse::Lost, now),
                        Status::Won => self.pulse(Pulse::Won, now),
                        _ if self.game.score() > old_score => self.pulse(Pulse::Eat, now),
                        _ => {}
                    }
                }
            }
            Advance::Interrupted => self.game.pause(),
            Advance::None => {}
        }
        if self.game.score() > self.settings.best {
            self.settings.best = self.game.score();
            self.save();
        }
    }

    fn header(&mut self, ui: &mut Ui, layout: &Layout) -> Action {
        if matches!(
            self.screen,
            Screen::Fih
                | Screen::Multiplayer
                | Screen::Wavelength
                | Screen::Ito
                | Screen::LocalPair(_)
                | Screen::Tennis
                | Screen::Nonograms
                | Screen::Sudoku
        ) {
            return Action::None;
        }
        let compact = self.screen == Screen::Mines && !self.mines.configuring;
        let left = if compact { 8.0 } else { layout.content.x };
        let right = if compact {
            screen_width() - 8.0
        } else {
            layout.content.right()
        };
        let top = if compact { 4.0 } else { 20.0 };
        if matches!(self.screen, Screen::Game | Screen::Mines) {
            if ui.icon_button(Icon::Back, Rect::new(left, top, 44.0, 44.0), false) {
                return Action::Home;
            }
            ui.centered(
                if self.screen == Screen::Mines {
                    "Minesweeper"
                } else {
                    "Snake"
                },
                Rect::new(screen_width() / 2.0 - 95.0, top, 190.0, 44.0),
                20.0,
                ui.theme.text,
                true,
            );
            if ui.icon_button(
                if self.screen == Screen::Mines {
                    Icon::Settings
                } else if self.game.status() == Status::Running {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                Rect::new(right - 44.0, top, 44.0, 44.0),
                false,
            ) {
                return if self.screen == Screen::Mines {
                    Action::NewMines
                } else if matches!(self.game.status(), Status::Lost | Status::Won) {
                    Action::NewGame
                } else {
                    Action::TogglePause
                };
            }
        } else {
            if ui.icon_button(Icon::Menu, Rect::new(left, top, 44., 44.), false) {
                return Action::Menu;
            }
            ui.heading("jarcade", left + 40.0, 50.0, 23.0, ui.theme.text);
            if ui.icon_button(
                if self.screen == Screen::Home {
                    Icon::Settings
                } else {
                    Icon::Back
                },
                Rect::new(right - 44.0, top, 44.0, 44.0),
                false,
            ) {
                return if self.screen == Screen::Home {
                    Action::Settings
                } else {
                    Action::Home
                };
            }
        }
        Action::None
    }

    fn home(&mut self, ui: &mut Ui, layout: &Layout, press: Option<Vec2>) -> Action {
        let x = layout.content.x;
        let width = layout.content.w;
        let tabs_y = if screen_height() < 500. {
            78.
        } else {
            let title_y = if screen_height() < 640. { 116. } else { 146. };
            ui.heading("Games", x, title_y, 44., ui.theme.text);
            title_y + 24.
        };
        let tab_width = (width - 8.) / 2.;
        if ui.tab(
            "Single player",
            Rect::new(x, tabs_y, tab_width, 42.),
            !self.multiplayer,
        ) {
            if self.multiplayer {
                self.home_pan = jarcade::board_pan::BoardPan::default();
            }
            self.multiplayer = false;
        }
        if ui.tab(
            "Multiplayer",
            Rect::new(x + tab_width + 8., tabs_y, tab_width, 42.),
            self.multiplayer,
        ) {
            if !self.multiplayer {
                self.home_pan = jarcade::board_pan::BoardPan::default();
            }
            self.multiplayer = true;
        }
        let card_y = tabs_y + if screen_height() < 500. { 54. } else { 68. };
        let count = if self.multiplayer { 11 } else { 6 };
        let grid = layout.game_grid_for(card_y, count);
        let viewport = Rect::new(x, card_y, width, (screen_height() - card_y - 16.).max(48.));
        let content_height = grid.content_height(count);
        // Header and category tabs are fixed; only the gallery scrolls.
        // Keyboard focus brings every launch card into view as well.
        if let Some(index) = ui.focused_item().and_then(|i| i.checked_sub(4))
            && index < count
        {
            let card = grid.card(index);
            let local_y = card.y - card_y;
            self.home_pan.offset.y = self
                .home_pan
                .offset
                .y
                .min(local_y)
                .max(local_y + card.h - viewport.h);
        }
        let (origin, tap) = online_view::page_input(
            &mut self.home_pan,
            &mut self.home_gesture_blocked,
            press,
            viewport,
            content_height,
        );
        let pointer = ui.override_pointer(tap);
        online_view::clip(Some(viewport));
        let mut chosen = Action::None;
        for index in 0..count {
            let mut card = grid.card(index);
            card.y += origin.y - card_y;
            let image_height = grid.image_height;
            if !self.settings.power_saver {
                rounded(
                    Rect::new(card.x, card.y + 3.0, card.w, card.h),
                    19.0,
                    color_u8!(237, 242, 235, 255),
                );
            }
            bordered(card, 19.0, ui.theme.line, ui.theme.bg);
            rounded(
                Rect::new(card.x + 1.0, card.y + 1.0, card.w - 2.0, image_height),
                18.0,
                if self.settings.power_saver {
                    BLACK
                } else {
                    color_u8!(243, 248, 238, 255)
                },
            );
            let preview_size = (image_height - 18.0).min(card.w - 24.0);
            let preview_rect = Rect::new(
                card.x + (card.w - preview_size) / 2.0,
                card.y + (image_height - preview_size) / 2.0,
                preview_size,
                preview_size,
            );
            if self.multiplayer {
                if index == 0 {
                    card_art::preview(ui, preview_rect);
                } else if index == 1 {
                    self.online.art.preview(ui, preview_rect);
                } else if index == 2 {
                    wavelength_view::preview(ui, preview_rect);
                } else if index == 3 {
                    wolves_art::preview(ui, preview_rect);
                } else if index == 4 {
                    codenames_view::preview(ui, preview_rect);
                } else if index == 5 {
                    telephone_view::preview(ui, preview_rect);
                } else if index == 6 {
                    ito_view::preview(ui, preview_rect);
                } else {
                    local_pair_view::preview(ui, PairKind::ALL[index - 7], preview_rect);
                }
            } else if index == 0 {
                self.preview.draw(preview_rect);
            } else if index == 1 {
                self.mines_preview.draw(preview_rect);
            } else if index == 2 {
                self.fih_preview.draw(preview_rect);
            } else if index == 3 {
                table_tennis_view::preview(ui, preview_rect);
            } else if index == 4 {
                nonograms_view::preview(ui, preview_rect);
            } else {
                self.sudoku.preview(ui, preview_rect);
            }
            let title = if self.multiplayer {
                [
                    "Coupe",
                    "Dicksit",
                    "Wavelength",
                    "Wolvesville",
                    "Codenames",
                    "Drawing Telephone",
                    "Ito",
                    "Cara a Cara",
                    "Mastermind",
                    "Jogo da Velha",
                    "Ligue 4",
                ][index]
            } else {
                [
                    "Snake",
                    "Minesweeper",
                    "Fih",
                    "Table tennis",
                    "Nonograms",
                    "Sudoku",
                ][index]
            };
            let title_size = (19.0 * (card.w - 24.0) / ui.text_width(title, 19.0, true)).min(19.0);
            ui.heading(
                title,
                card.x + 12.0,
                card.y + image_height + 25.0,
                title_size,
                ui.theme.text,
            );
            ui.label(
                if self.multiplayer {
                    [
                        "Bluff · 2–6",
                        "Stories · 3–8",
                        "Local · 2+",
                        "6–16",
                        "4–16",
                        "Draw · 3–12",
                        "Local · 2–10",
                        "Local · 2",
                        "Local · 2",
                        "Local · 2",
                        "Local · 2",
                    ][index]
                } else {
                    ["Classic", "Puzzle", "Pet", "vs CPU", "Puzzle", "Variants"][index]
                },
                card.x + 12.0,
                card.y + image_height + 46.0,
                12.0,
                ui.theme.muted,
            );
            let play = Rect::new(
                card.right() - 44.0,
                card.y + image_height + 26.0,
                32.0,
                32.0,
            );
            if self.settings.power_saver {
                bordered(play, 16.0, ui.theme.accent, BLACK);
            } else {
                rounded(play, 16.0, GREEN);
            }
            draw_icon(
                Icon::Play,
                play.center(),
                if self.settings.power_saver {
                    ui.theme.accent
                } else {
                    WHITE
                },
            );
            if ui.hit(card) {
                chosen = if self.multiplayer {
                    if index == 0 {
                        Action::NewCourt
                    } else if index == 1 {
                        Action::NewReverie
                    } else if index == 2 {
                        Action::NewWavelength
                    } else if index == 3 {
                        Action::NewWolves
                    } else if index == 4 {
                        Action::NewCodenames
                    } else if index == 5 {
                        Action::NewTelephone
                    } else if index == 6 {
                        Action::NewIto
                    } else {
                        Action::NewPair(PairKind::ALL[index - 7])
                    }
                } else if index == 0 {
                    Action::NewGame
                } else if index == 1 {
                    Action::NewMines
                } else if index == 2 {
                    Action::NewFih
                } else if index == 3 {
                    Action::NewTennis
                } else if index == 4 {
                    Action::NewNonograms
                } else {
                    Action::NewSudoku
                };
                break;
            }
        }
        online_view::clip(None);
        ui.override_pointer(pointer);
        if content_height > viewport.h {
            let thumb_h = (viewport.h * viewport.h / content_height).max(24.);
            let thumb_y = viewport.y
                + self.home_pan.offset.y / (content_height - viewport.h) * (viewport.h - thumb_h);
            rounded(
                Rect::new(viewport.right() - 3., thumb_y, 3., thumb_h),
                1.5,
                ui.theme.muted,
            );
        }
        if !matches!(chosen, Action::None) {
            return chosen;
        }
        if !ui.keyboard_focus && is_key_pressed(KeyCode::Enter) {
            return if self.multiplayer {
                Action::NewCourt
            } else {
                Action::NewGame
            };
        }
        Action::None
    }

    fn settings_page(&mut self, ui: &mut Ui, layout: &Layout) -> Action {
        let width = layout.content.w.min(620.0);
        let x = if screen_width() > 700.0 {
            (screen_width() - width) / 2.0
        } else {
            layout.content.x
        };
        let y = if screen_height() < 500.0 {
            100.0
        } else {
            146.0
        };
        ui.heading("Settings", x, y, 40.0, ui.theme.text);
        let short = screen_height() < 500.0;
        let row_height = if short {
            ((screen_height() - y - 36.) / 3.).clamp(44., 58.)
        } else {
            88.
        };
        let panel = Rect::new(
            x,
            y + if short { 14.0 } else { 30.0 },
            width,
            row_height * 3.0 + 20.0,
        );
        bordered(panel, 24.0, ui.theme.line, ui.theme.bg);
        for (index, title, subtitle, on, enabled, action) in [
            (
                0,
                "Power saver",
                "Black screen. Less motion.",
                self.settings.power_saver,
                true,
                Action::TogglePower,
            ),
            (
                1,
                "Haptics",
                if self.haptics_supported {
                    "Taps and game events."
                } else {
                    "Unavailable on this device."
                },
                self.settings.haptics,
                self.haptics_supported,
                Action::ToggleHaptics,
            ),
            (
                2,
                "FPS counter",
                "Bottom-right corner.",
                self.settings.show_fps,
                true,
                Action::ToggleFps,
            ),
        ] {
            let row = Rect::new(
                panel.x + 20.0,
                panel.y + 10.0 + index as f32 * row_height,
                panel.w - 40.0,
                row_height - 4.0,
            );
            if ui.toggle(title, if short { "" } else { subtitle }, row, on, enabled) {
                return action;
            }
            if index < 2 {
                draw_line(
                    row.x,
                    row.y + row_height - 2.0,
                    row.right(),
                    row.y + row_height - 2.0,
                    1.0,
                    ui.theme.line,
                );
            }
        }
        if self.save_failed {
            ui.label(
                "Couldn't save your settings.",
                x,
                panel.bottom() + 28.0,
                13.0,
                CORAL,
            );
        }
        Action::None
    }

    fn game_page(&mut self, ui: &mut Ui, layout: &Layout) -> Action {
        let board = layout.board;
        if !layout.landscape && screen_height() < 540. {
            ui.centered(
                &format!(
                    "Score {}   ·   Best {}",
                    self.game.score(),
                    self.settings.best
                ),
                Rect::new(board.x, board.y - 28., board.w, 22.),
                12.,
                ui.theme.muted,
                true,
            );
        } else if !layout.landscape {
            ui.label("SCORE", board.x, board.y - 58.0, 11.0, ui.theme.muted);
            ui.heading(
                &format!("{:02}", self.game.score()),
                board.x,
                board.y - 19.0,
                34.0,
                ui.theme.text,
            );
            ui.label(
                "BEST",
                board.right() - 60.0,
                board.y - 58.0,
                11.0,
                ui.theme.muted,
            );
            ui.heading(
                &format!("{:02}", self.settings.best),
                board.right() - 60.0,
                board.y - 19.0,
                34.0,
                ui.theme.muted,
            );
        } else {
            ui.centered(
                &format!(
                    "{:02}  /  best {:02}",
                    self.game.score(),
                    self.settings.best
                ),
                Rect::new(board.right() + 20.0, 88.0, 180.0, 30.0),
                15.0,
                ui.theme.muted,
                false,
            );
        }
        let smooth = !self.settings.power_saver
            && matches!(self.game.status(), Status::Running | Status::Paused);
        body_path(
            if smooth { &self.previous_body } else { &[] },
            self.game.body(),
            self.clock.fraction(),
            &mut self.render_body,
        );
        let unit = board.w / f32::from(jarcade::snake::BOARD_SIZE);
        for point in &mut self.render_body {
            *point = board.point() + (*point + vec2(0.5, 0.5)) * unit;
        }
        draw_board(
            board,
            &self.render_body,
            self.game.food(),
            self.game.direction(),
            smooth.then(|| {
                head_heading(
                    &self.previous_body,
                    self.game.direction(),
                    self.clock.fraction(),
                )
            }),
            &ui.theme,
        );
        if self.game.status() != Status::Running {
            let ready = self.game.status() == Status::Ready;
            let overlay_width = (board.w - 24.0).min(286.0);
            let overlay_height = if board.w < 205.0 { 122.0 } else { 148.0 };
            let overlay = Rect::new(
                board.center().x - overlay_width / 2.0,
                board.center().y - overlay_height / 2.0,
                overlay_width,
                overlay_height,
            );
            bordered(overlay, 22.0, ui.theme.line, ui.theme.bg);
            let title = match self.game.status() {
                Status::Ready => "Ready?",
                Status::Paused => "Paused",
                Status::Lost => "Game over",
                Status::Won => "You did it!",
                Status::Running => unreachable!(),
            };
            ui.centered(
                title,
                Rect::new(overlay.x, overlay.y + 14.0, overlay.w, 38.0),
                if board.w < 240.0 { 20.0 } else { 25.0 },
                ui.theme.text,
                true,
            );
            let retry = matches!(self.game.status(), Status::Lost | Status::Won);
            if ui.button(
                if retry {
                    "Play again"
                } else if ready {
                    "Start"
                } else {
                    "Resume"
                },
                Rect::new(
                    overlay.x + 16.0,
                    overlay.bottom() - 66.0,
                    overlay.w - 32.0,
                    48.0,
                ),
                true,
            ) {
                return if retry {
                    Action::NewGame
                } else {
                    Action::TogglePause
                };
            }
        }
        if !self.settings.power_saver
            && !layout.landscape
            && layout.controls.y + 90.0 < screen_height()
        {
            ui.centered(
                "Swipe to steer",
                Rect::new(board.x, layout.controls.y + 67.0, board.w, 22.0),
                12.0,
                ui.theme.muted,
                false,
            );
        }
        Action::None
    }
}

fn seed() -> u64 {
    (miniquad::date::now() * 1_000_000.0) as u64
}

#[macroquad::main(window_conf)]
async fn main() {
    simulate_mouse_with_touch(false);
    platform::configure_display();
    let mut ui = Ui::new();
    let mut app = App::new(&ui);
    if let Some(route) = platform::route_load() {
        app.navigate(route, &mut ui, get_time());
    }
    let mut last_route = app.route();
    platform::route_sync(last_route, true);
    platform::appearance(app.settings.power_saver, app.screen == Screen::Wavelength);
    let timer = platform::WakeTimer::new();
    let subscriber = macroquad::input::utils::register_input_subscriber();
    let mut input = SnakeInput::default();
    let mut last_announcement = None;
    loop {
        let frame_start = get_time();
        let layout = Layout::new(screen_width(), screen_height());
        input.configure(
            &layout,
            screen_dpi_scale(),
            app.screen == Screen::Game
                && matches!(app.game.status(), Status::Ready | Status::Running),
        );
        macroquad::input::utils::repeat_all_miniquad_input(&mut input, subscriber);
        if let Some(navigation) = platform::route_poll() {
            if let Some(route) = Route::parse(&navigation.route) {
                if route != app.route() {
                    app.navigate(route, &mut ui, frame_start);
                }
                last_route = app.route();
                if route != last_route {
                    platform::route_sync(last_route, true);
                }
            }
            app.set_drawer(navigation.drawer, &mut ui);
            input.cancel();
            last_announcement = None;
        }
        let web_interrupted = platform::web_interrupted();
        if input.interrupted || web_interrupted {
            app.game.pause();
            app.tennis.interrupt();
            app.fih.interrupt();
            if app.screen == Screen::Wavelength {
                app.wavelength.interrupt();
            }
            if app.screen == Screen::Ito {
                app.ito.interrupt();
            }
            if matches!(app.screen, Screen::LocalPair(_)) {
                app.pair.interrupt();
            }
            input.cancel();
            app.mines.cancel_gesture();
            app.nonograms.cancel_gesture();
            app.sudoku.cancel_gesture();
            app.online.cancel_gesture();
            app.home_pan.cancel();
            input.interrupted = false;
        }
        if app.screen == Screen::Multiplayer {
            app.online.poll();
        }
        if app.screen == Screen::Wavelength {
            app.wavelength.poll();
        }
        if app.screen == Screen::Ito {
            app.ito.poll();
        }
        ui.begin(app.settings.power_saver, input.pointer);
        if is_key_pressed(KeyCode::Back) {
            platform::route_drawer(!app.drawer);
            if !cfg!(target_arch = "wasm32") {
                app.set_drawer(!app.drawer, &mut ui);
            }
            input.cancel();
        }
        if app.drawer {
            clear_background(ui.theme.bg);
            let action = if is_key_pressed(KeyCode::Escape) {
                Action::CloseMenu
            } else {
                app.sidebar(&mut ui)
            };
            ui.end();
            let changed = app.act(action, &mut ui, frame_start);
            if last_route != app.route() {
                last_route = app.route();
                platform::route_sync(last_route, false);
            }
            platform::announce(&format!(
                "Jarcade. Navigation. {}. Continue, Single player, Multiplayer{}.",
                app.route().title(),
                if app.route().is_menu() {
                    ", Settings"
                } else {
                    ""
                }
            ));
            last_announcement = None;
            timer.arm(changed.then_some(0.));
            next_frame().await;
            continue;
        }
        let mut action = Action::None;
        if is_key_pressed(KeyCode::Escape) {
            action = if app.screen == Screen::Fih {
                ui.reset_focus();
                if app.fih.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if app.screen == Screen::Multiplayer {
                if app.online.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if matches!(app.screen, Screen::LocalPair(_)) {
                if app.pair.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if app.screen == Screen::Ito {
                if app.ito.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if app.screen == Screen::Wavelength {
                if app.wavelength.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if app.screen == Screen::Tennis && app.tennis.needs_frame() {
                app.tennis.interrupt();
                Action::None
            } else if app.screen == Screen::Nonograms {
                if app.nonograms.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if app.screen == Screen::Sudoku {
                if app.sudoku.back() {
                    Action::Home
                } else {
                    Action::None
                }
            } else if app.screen == Screen::Game && app.game.status() == Status::Running {
                Action::TogglePause
            } else {
                Action::Home
            };
        }
        if matches!(action, Action::None) {
            app.game_input(&ui, &mut input, frame_start);
        }
        clear_background(ui.theme.bg);
        let nav_action = app.header(&mut ui, &layout);
        if matches!(action, Action::None) {
            action = nav_action;
        }
        if app.screen == Screen::Game {
            app.game_controls(&mut ui, &layout, frame_start);
        }
        app.advance(frame_start);
        // These static games can change state while drawing keyboard controls.
        // Present the resulting board once before returning to event-driven idle.
        let pair_revision = app.pair.revision;
        let page_action = match app.screen {
            Screen::Multiplayer => {
                if app.online.draw(&mut ui, input.pointer, &input.keys) {
                    Action::Home
                } else {
                    Action::None
                }
            }
            Screen::LocalPair(_) => {
                let (back, pulse) = app.pair.draw(&mut ui, input.pointer, &input.keys);
                if let Some(pulse) = pulse {
                    app.pulse(pulse, frame_start);
                }
                if back { Action::Home } else { Action::None }
            }
            Screen::Ito => {
                let (back, pulse) = app.ito.draw(&mut ui, input.pointer, &input.keys);
                if let Some(pulse) = pulse {
                    app.pulse(pulse, frame_start);
                }
                if back { Action::Home } else { Action::None }
            }
            Screen::Wavelength => {
                let (back, pulse) = app.wavelength.draw(&mut ui, input.pointer, &input.keys);
                if let Some(pulse) = pulse {
                    app.pulse(pulse, frame_start);
                }
                if back { Action::Home } else { Action::None }
            }
            Screen::Tennis => {
                let (back, pulse) =
                    app.tennis
                        .draw(&mut ui, input.pointer, &input.keys, frame_start);
                if let Some(pulse) = pulse {
                    app.pulse(pulse, frame_start);
                }
                if back { Action::Home } else { Action::None }
            }
            Screen::Home => app.home(&mut ui, &layout, input.pointer),
            Screen::Nonograms => {
                if let Some(pulse) = app.nonograms.draw(&mut ui, input.pointer) {
                    app.pulse(pulse, frame_start);
                }
                if let Some(data) = app.nonograms.take_save() {
                    let saved = platform::save_nonograms(&data);
                    app.nonograms.saved(saved);
                }
                if app.nonograms.take_home() {
                    Action::Home
                } else {
                    Action::None
                }
            }
            Screen::Sudoku => {
                if let Some(pulse) = app.sudoku.draw(&mut ui, input.pointer, &input.keys) {
                    app.pulse(pulse, frame_start);
                }
                if let Some(data) = app.sudoku.take_save() {
                    let saved = platform::save_sudoku(&data);
                    app.sudoku.saved(saved);
                }
                if app.sudoku.take_home() {
                    Action::Home
                } else {
                    Action::None
                }
            }
            Screen::Settings => app.settings_page(&mut ui, &layout),
            Screen::Game => app.game_page(&mut ui, &layout),
            Screen::Fih => {
                if let Some(pulse) = app.fih.draw(&mut ui, input.pointer, frame_start) {
                    app.pulse(pulse, frame_start);
                }
                match app.fih.take_nav() {
                    Nav::None => Action::None,
                    Nav::Arcade => Action::Home,
                }
            }
            Screen::Mines => {
                if let Some(pulse) = app.mines.draw(&mut ui) {
                    app.pulse(pulse, frame_start);
                }
                Action::None
            }
        };
        if matches!(action, Action::None) {
            action = page_action;
        }
        let continuous = (app.screen == Screen::Game && app.game.status() == Status::Running)
            || (app.screen == Screen::Tennis && app.tennis.needs_frame())
            || (app.screen == Screen::Mines && app.mines.needs_frame())
            || (app.screen == Screen::Nonograms && app.nonograms.needs_frame())
            || (app.screen == Screen::Sudoku && app.sudoku.needs_frame())
            || (app.screen == Screen::Home && app.home_pan.active())
            || (app.screen == Screen::Multiplayer && app.online.needs_frame())
            || (app.screen == Screen::Wavelength && app.wavelength.needs_frame())
            || (app.screen == Screen::Ito && app.ito.needs_frame())
            || (matches!(app.screen, Screen::LocalPair(_)) && app.pair.needs_frame())
            || (app.screen == Screen::Fih
                && app.fih.needs_frame(frame_start, app.settings.power_saver));
        let fps = app.fps.record(frame_start, continuous);
        if app.settings.show_fps {
            let text = if continuous {
                fps.map_or_else(|| "— FPS".into(), |n| format!("{n} FPS"))
            } else {
                "FPS · idle".into()
            };
            let badge = Rect::new(
                screen_width() - 86.0,
                if app.screen == Screen::Fih {
                    screen_height() - 25.0
                } else if matches!(
                    app.screen,
                    Screen::Wavelength
                        | Screen::Ito
                        | Screen::LocalPair(_)
                        | Screen::Tennis
                        | Screen::Multiplayer
                        | Screen::Nonograms
                        | Screen::Sudoku
                ) {
                    screen_height() - 18.0
                } else {
                    screen_height() - 27.0
                },
                78.0,
                if matches!(
                    app.screen,
                    Screen::Wavelength
                        | Screen::Ito
                        | Screen::LocalPair(_)
                        | Screen::Tennis
                        | Screen::Multiplayer
                        | Screen::Nonograms
                        | Screen::Sudoku
                ) {
                    14.0
                } else {
                    21.0
                },
            );
            rounded(badge, 8.0, ui.theme.bg);
            ui.centered(&text, badge, 11.0, ui.theme.muted, false);
        }
        ui.end();
        let changed = app.act(action, &mut ui, frame_start);
        if last_route != app.route() {
            last_route = app.route();
            platform::route_sync(last_route, false);
        }
        if ui.activated {
            app.pulse(Pulse::Tap, frame_start);
        }
        let state = (
            app.screen,
            app.game.status(),
            app.game.score(),
            app.settings.power_saver,
            app.settings.haptics,
            app.settings.show_fps,
            app.mines.game.status(),
            app.mines.game.revealed(),
            app.mines.game.flags(),
            app.mines.flag_mode,
            app.mines.selected,
            (
                app.mines.configuring,
                app.mines.size,
                app.mines.zoom_percent(),
                (
                    app.fih.revision,
                    app.online.revision,
                    app.wavelength.revision,
                    (app.ito.revision, app.pair.revision),
                    app.tennis.game.revision,
                    (
                        app.nonograms.revision,
                        (app.screen == Screen::Nonograms)
                            .then(|| ui.focused_item())
                            .flatten(),
                    ),
                    (
                        app.sudoku.revision,
                        (app.screen == Screen::Sudoku)
                            .then(|| ui.focused_item())
                            .flatten(),
                    ),
                    app.multiplayer,
                ),
                app.fih.round.as_ref().map(|r| {
                    (
                        r.kind,
                        r.score,
                        r.paused,
                        r.finished,
                        r.remaining().ceil() as u32,
                    )
                }),
            ),
        );
        if last_announcement != Some(state) {
            let message = match app.screen {
                Screen::Multiplayer => app.online.announcement(),
                Screen::Wavelength => app.wavelength.announcement(),
                Screen::Ito => app.ito.announcement(),
                Screen::LocalPair(_) => app.pair.announcement(),
                Screen::Tennis => app.tennis.announcement(),
                Screen::Nonograms => app.nonograms.announcement(ui.focused_item()),
                Screen::Sudoku => app.sudoku.announcement(ui.focused_item()),
                Screen::Home => {
                    if app.multiplayer {
                        "Jarcade. Multiplayer. Select Coupe, Dicksit, Wavelength, Wolvesville, Codenames, Drawing Telephone, Ito, Cara a Cara, Mastermind, Jogo da Velha, or Ligue 4. Coupe, Dicksit, Wolvesville, Codenames and Drawing Telephone use online rooms; Wavelength, Ito, Cara a Cara, Mastermind, Jogo da Velha and Ligue 4 are local on this device.".to_owned()
                    } else {
                        "Jarcade. Games. Select Snake, Minesweeper, Fih, Table tennis, Nonograms, or Sudoku to play."
                            .to_owned()
                    }
                }
                Screen::Settings => format!(
                    "Jarcade. Settings. Power saver {}. Haptics {}. FPS counter {}.",
                    if app.settings.power_saver {
                        "on"
                    } else {
                        "off"
                    },
                    if app.settings.haptics { "on" } else { "off" },
                    if app.settings.show_fps { "on" } else { "off" }
                ),
                Screen::Mines => app.mines.announcement(),
                Screen::Fih => app.fih.announcement(),
                Screen::Game => format!(
                    "Jarcade. Snake. {:?}. Score {}. Best {}.",
                    app.game.status(),
                    app.game.score(),
                    app.settings.best
                ),
            };
            platform::announce(&message);
            last_announcement = Some(state);
        }
        let delay = if changed
            || ui.activated
            || (matches!(app.screen, Screen::LocalPair(_)) && app.pair.revision != pair_revision)
            || (app.screen == Screen::Mines && app.mines.needs_frame())
            || (app.screen == Screen::Nonograms && app.nonograms.needs_frame())
            || (app.screen == Screen::Sudoku && app.sudoku.needs_frame())
            || (app.screen == Screen::Home && app.home_pan.active())
        {
            Some(0.0)
        } else if app.screen == Screen::Game && app.game.status() == Status::Running {
            if app.settings.power_saver {
                Some((app.clock.until_tick() - (get_time() - frame_start)).max(0.001))
            } else {
                // Queue the next vsync/rAF directly, without a second timer or FPS cap.
                Some(0.0)
            }
        } else if (app.screen == Screen::Tennis && app.tennis.needs_frame())
            || (app.screen == Screen::Wavelength && app.wavelength.needs_frame())
            || (app.screen == Screen::Ito && app.ito.needs_frame())
            || (matches!(app.screen, Screen::LocalPair(_)) && app.pair.needs_frame())
            || (app.screen == Screen::Multiplayer && app.online.needs_frame())
        {
            Some(if app.settings.power_saver {
                1.0 / 30.0
            } else {
                0.0
            })
        } else if app.screen == Screen::Multiplayer {
            app.online.delay()
        } else if app.screen == Screen::Fih {
            app.fih.delay(frame_start, app.settings.power_saver)
        } else {
            None
        };
        timer.arm(delay);
        next_frame().await;
    }
}
