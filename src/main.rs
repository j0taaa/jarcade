mod card_art;
mod codenames_view;
mod fih_art;
mod fih_character;
mod fih_view;
mod game_view;
mod mines_view;
mod nonograms_view;
mod online_net;
mod online_style;
mod online_view;
mod platform;
mod table_tennis_view;
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
    Tennis,
    Nonograms,
}
#[derive(Clone, Copy)]
enum Action {
    None,
    Home,
    Settings,
    NewGame,
    NewMines,
    NewFih,
    NewCourt,
    NewReverie,
    NewWolves,
    NewCodenames,
    NewWavelength,
    NewTennis,
    NewNonograms,
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
    tennis: table_tennis_view::TennisPage,
    nonograms: nonograms_view::NonogramsPage,
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
            online: online_view::OnlinePage::new(),
            wavelength: wavelength_view::WavelengthPage::new(seed()),
            tennis: table_tennis_view::TennisPage::new(seed()),
            nonograms: nonograms_view::NonogramsPage::new(platform::load_nonograms()),
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
            Action::Home => {
                self.tennis.interrupt();
                self.online.suspend();
                if self.screen == Screen::Wavelength {
                    self.wavelength.interrupt();
                }
                self.fih.interrupt();
                self.fih_preview = FihPreview::new(&self.fih.pet, self.settings.power_saver);
                self.mines.cancel_gesture();
                self.nonograms.cancel_gesture();
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
            Action::NewCourt | Action::NewReverie | Action::NewWolves | Action::NewCodenames => {
                self.game.pause();
                self.online.enter(
                    if matches!(action, Action::NewCourt) {
                        jarcade::multiplayer::GameKind::Court
                    } else if matches!(action, Action::NewReverie) {
                        jarcade::multiplayer::GameKind::Reverie
                    } else if matches!(action, Action::NewWolves) {
                        jarcade::multiplayer::GameKind::Wolves
                    } else {
                        jarcade::multiplayer::GameKind::Codenames
                    },
                    None,
                );
                self.screen = Screen::Multiplayer;
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
                | Action::NewWavelength
                | Action::NewTennis
                | Action::NewNonograms
        ) {
            self.home_pan.cancel();
            self.nonograms.cancel_gesture();
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
                | Screen::Tennis
                | Screen::Nonograms
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
            logo(Rect::new(left, 27.0, 30.0, 30.0), ui.theme.accent);
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
        let count = 5;
        let grid = layout.game_grid_for(card_y, count);
        let viewport = Rect::new(x, card_y, width, (screen_height() - card_y - 16.).max(48.));
        let content_height = grid.content_height(count);
        // Header and category tabs are fixed; only the gallery scrolls.
        // Keyboard focus brings every launch card into view as well.
        if let Some(index) = ui.focused_item().and_then(|i| i.checked_sub(3))
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
                } else {
                    codenames_view::preview(ui, preview_rect);
                }
            } else if index == 0 {
                self.preview.draw(preview_rect);
            } else if index == 1 {
                self.mines_preview.draw(preview_rect);
            } else if index == 2 {
                self.fih_preview.draw(preview_rect);
            } else if index == 3 {
                table_tennis_view::preview(ui, preview_rect);
            } else {
                nonograms_view::preview(ui, preview_rect);
            }
            let title = if self.multiplayer {
                ["Coupe", "Dicksit", "Wavelength", "Wolvesville", "Codenames"][index]
            } else {
                ["Snake", "Minesweeper", "Fih", "Table tennis", "Nonograms"][index]
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
                    ["Bluff · 2–6", "Stories · 3–8", "Local · 2+", "6–16", "4–16"][index]
                } else {
                    ["Classic", "Puzzle", "Pet", "vs CPU", "Puzzle"][index]
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
                    } else {
                        Action::NewCodenames
                    }
                } else if index == 0 {
                    Action::NewGame
                } else if index == 1 {
                    Action::NewMines
                } else if index == 2 {
                    Action::NewFih
                } else if index == 3 {
                    Action::NewTennis
                } else {
                    Action::NewNonograms
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
    if let Some((game, code)) = platform::invite() {
        app.online.enter(game, Some(code));
        app.screen = Screen::Multiplayer;
        app.multiplayer = true;
    } else if platform::launch_wavelength() {
        app.wavelength.enter();
        app.screen = Screen::Wavelength;
        app.multiplayer = true;
    }
    if app.screen == Screen::Home && platform::launch_table_tennis() {
        app.tennis.enter(seed());
        app.screen = Screen::Tennis;
    }
    if app.screen == Screen::Home && platform::launch_nonograms() {
        app.nonograms.enter();
        app.screen = Screen::Nonograms;
    }
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
        let web_interrupted = platform::web_interrupted();
        if input.interrupted || web_interrupted {
            app.game.pause();
            app.tennis.interrupt();
            app.fih.interrupt();
            if app.screen == Screen::Wavelength {
                app.wavelength.interrupt();
            }
            input.cancel();
            app.mines.cancel_gesture();
            app.nonograms.cancel_gesture();
            app.home_pan.cancel();
            input.interrupted = false;
        }
        if app.screen == Screen::Multiplayer {
            app.online.poll();
        }
        if app.screen == Screen::Wavelength {
            app.wavelength.poll();
        }
        ui.begin(app.settings.power_saver, input.pointer);
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
        let page_action = match app.screen {
            Screen::Multiplayer => {
                if app.online.draw(&mut ui, input.pointer) {
                    Action::Home
                } else {
                    Action::None
                }
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
            || (app.screen == Screen::Home && app.home_pan.active())
            || (app.screen == Screen::Multiplayer && app.online.needs_frame())
            || (app.screen == Screen::Wavelength && app.wavelength.needs_frame())
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
                    Screen::Wavelength | Screen::Tennis | Screen::Multiplayer | Screen::Nonograms
                ) {
                    screen_height() - 18.0
                } else {
                    screen_height() - 27.0
                },
                78.0,
                if matches!(
                    app.screen,
                    Screen::Wavelength | Screen::Tennis | Screen::Multiplayer | Screen::Nonograms
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
                    app.tennis.game.revision,
                    app.nonograms.revision,
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
                Screen::Tennis => app.tennis.announcement(),
                Screen::Nonograms => app.nonograms.announcement(),
                Screen::Home => {
                    if app.multiplayer {
                        "Jarcade. Multiplayer. Select Coupe, Dicksit, Wavelength, Wolvesville, or Codenames. Coupe, Dicksit, Wolvesville and Codenames use online rooms; Wavelength is local on this device.".to_owned()
                    } else {
                        "Jarcade. Games. Select Snake, Minesweeper, Fih, Table tennis, or Nonograms to play."
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
            || (app.screen == Screen::Mines && app.mines.needs_frame())
            || (app.screen == Screen::Nonograms && app.nonograms.needs_frame())
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
