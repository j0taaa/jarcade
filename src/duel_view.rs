//! Original vectors, simultaneous input and private pass-and-play naval boards.
use crate::{
    online_style::primary,
    online_view::{clip, fit, page_input, wrap},
    platform,
    ui::{CORAL, Icon, Ui, bordered, rounded},
};
use jarcade::{
    air_hockey::{self as ah, Event, Hockey, Phase as HPhase},
    battleship::{self as bs, Phase as NPhase, Result as ShotResult, Resume, Ship, Tile},
    board_pan::BoardPan,
    duel::{Kind, State},
    feedback::Pulse,
    layout::touch_point,
};
use macroquad::{miniquad::KeyMods, prelude::*};
const BLUE: Color = color_u8!(48, 115, 188, 255);
pub struct DuelPage {
    state: State,
    kind: Kind,
    pub revision: u64,
    last: Option<f64>,
    fingers: [Option<u64>; 2],
    mouse: Option<usize>,
    size: Vec2,
    help: bool,
    reset: bool,
    pan: BoardPan,
    blocked: bool,
    selected: usize,
    vertical: bool,
    anchor: Option<usize>,
    own: bool,
    end_player: usize,
    save_failed: bool,
    last_hit: f64,
}
impl DuelPage {
    pub fn new() -> Self {
        Self {
            state: platform::load_duel(),
            kind: Kind::Hockey,
            revision: 0,
            last: None,
            fingers: [None; 2],
            mouse: None,
            size: Vec2::ZERO,
            help: false,
            reset: false,
            pan: BoardPan::default(),
            blocked: false,
            selected: 0,
            vertical: false,
            anchor: None,
            own: false,
            end_player: 0,
            save_failed: false,
            last_hit: -10.,
        }
    }
    fn save(&mut self) {
        self.revision += 1;
        self.save_failed = !platform::save_duel(&self.state);
    }
    pub fn enter(&mut self, kind: Kind) {
        self.interrupt();
        self.kind = kind;
        self.help = false;
        self.reset = false;
        self.pan = BoardPan::default();
        self.anchor = None;
        self.own = false;
        self.selected = 0;
    }
    pub fn interrupt(&mut self) {
        self.state.interrupt();
        self.last = None;
        self.fingers = [None; 2];
        self.mouse = None;
        self.pan.cancel();
        self.blocked = true;
        self.anchor = None;
        self.save();
    }
    pub fn needs_frame(&self) -> bool {
        !self.help
            && !self.reset
            && ((self.kind == Kind::Hockey && self.state.hockey.phase == HPhase::Playing)
                || self.pan.active())
    }
    pub fn back(&mut self) -> bool {
        if self.help || self.reset {
            self.help = false;
            self.reset = false;
            self.pan = BoardPan::default();
            self.revision += 1;
            false
        } else {
            self.interrupt();
            true
        }
    }
    pub fn announcement(&self) -> String {
        let mut s = format!(
            "Jarcade. {}. Local, duas pessoas no mesmo aparelho. ",
            self.kind.title()
        );
        if self.help {
            return s + &self.rules().join(" ");
        }
        if self.reset {
            return s + "Confirmar reinício. Cancelar ou Reiniciar.";
        }
        if self.kind == Kind::Hockey {
            s.push_str(&format!("{:?}. Placar {} a {}. Partidas {} a {}. Primeiro a 7. WASD: jogador 1; setas: jogador 2. Space inicia ou pausa.",self.state.hockey.phase,self.state.hockey.scores[0],self.state.hockey.scores[1],self.state.hockey.matches[0],self.state.hockey.matches[1]));
        } else {
            let g = &self.state.naval;
            s.push_str(&format!(
                "{:?}. Jogador {}. Placar {} a {}. ",
                g.phase,
                g.player() + 1,
                g.scores[0],
                g.scores[1]
            ));
            if let Some(v) = g.view() {
                if matches!(g.phase, NPhase::Setup(_)) {
                    s.push_str(&format!(
                        "Escolhido: {}. Frota: {:?}. ",
                        bs::NAMES[self.selected],
                        g.placements().unwrap()
                    ));
                } else {
                    s.push_str(&format!(
                        "{}: {:?}. ",
                        if self.own { "Minha frota" } else { "Alvos" },
                        if self.own { &v.own } else { &v.target }
                    ));
                    if let Some((cell, result)) = v.last {
                        s.push_str(&format!(
                            "Último tiro: {}. {:?}. ",
                            coordinate(cell),
                            result
                        ));
                    }
                }
                if let Some(cell) = self.anchor {
                    s.push_str(&format!("Selecionado {}. ", coordinate(cell)));
                }
            }
        }
        if self.save_failed {
            s.push_str("Progresso não salvo.");
        }
        s
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        now: f64,
    ) -> (bool, Option<Pulse>) {
        let size = vec2(screen_width(), screen_height());
        if size != self.size {
            self.interrupt();
            self.size = size;
        }
        ui.theme.accent = if self.kind == Kind::Hockey {
            CORAL
        } else {
            BLUE
        };
        let w = (size.x - 32.).min(1040.);
        let b = Rect::new((size.x - w) / 2., 66., w, size.y - 82.);
        if ui.icon_button(Icon::Back, Rect::new(b.x, 8., 44., 44.), false) && self.back() {
            return (true, None);
        }
        fit(
            ui,
            self.kind.title(),
            Rect::new(b.x + 50., 8., w - 150., 44.),
            22.,
            ui.theme.text,
            true,
        );
        if ui.button("↻", Rect::new(b.right() - 94., 8., 44., 44.), false) {
            self.interrupt();
            self.reset = true;
            self.help = false;
        }
        if ui.button("?", Rect::new(b.right() - 44., 8., 44., 44.), false) {
            self.interrupt();
            self.help = !self.help;
            self.reset = false;
            self.pan = BoardPan::default();
        }
        if self.help {
            self.draw_rules(ui, press, b);
            return (false, None);
        }
        if self.reset {
            let r = Rect::new(
                b.center().x - b.w.min(420.) / 2.,
                b.center().y - 82.,
                b.w.min(420.),
                164.,
            );
            bordered(r, 20., ui.theme.line, ui.theme.bg);
            fit(
                ui,
                "Reiniciar partida e placar?",
                Rect::new(r.x + 12., r.y + 20., r.w - 24., 40.),
                18.,
                ui.theme.text,
                true,
            );
            if ui.button(
                "Cancelar",
                Rect::new(r.x + 12., r.bottom() - 60., (r.w - 32.) / 2., 48.),
                false,
            ) {
                self.reset = false;
                self.revision += 1;
            }
            if primary(
                ui,
                "Reiniciar",
                Rect::new(r.center().x + 4., r.bottom() - 60., (r.w - 32.) / 2., 48.),
                true,
            ) {
                if self.kind == Kind::Hockey {
                    self.state.hockey = Hockey::default();
                } else {
                    self.state.naval = bs::Naval::default();
                }
                self.reset = false;
                self.anchor = None;
                self.pan = BoardPan::default();
                self.save();
            }
            return (false, None);
        }
        let pulse = if self.kind == Kind::Hockey {
            self.draw_hockey(ui, press, keys, b, now)
        } else {
            self.draw_naval(ui, press, keys, b)
        };
        if self.save_failed {
            fit(
                ui,
                "Progresso não salvo",
                Rect::new(b.x, 54., b.w, 12.),
                10.,
                CORAL,
                false,
            );
        }
        (false, pulse)
    }
    fn status(&self, ui: &Ui, r: Rect, label: &str, scores: [u32; 2]) {
        fit(
            ui,
            label,
            Rect::new(r.x, r.y, r.w - 100., 36.),
            16.,
            ui.theme.text,
            true,
        );
        let p = Rect::new(r.right() - 96., r.y, 96., 36.);
        bordered(p, 18., ui.theme.line, ui.theme.bg);
        fit(
            ui,
            &format!("{} · {}", scores[0], scores[1]),
            p,
            17.,
            ui.theme.text,
            true,
        );
    }
    fn draw_hockey(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        b: Rect,
        now: f64,
    ) -> Option<Pulse> {
        let l = hockey_layout(b);
        let dt = self.last.map_or(0., |last| (now - last).max(0.) as f32);
        self.last = Some(now);
        let running = self.state.hockey.phase == HPhase::Playing;
        if running {
            self.hockey_input(l, press, keys, dt);
        }
        let mut pulse = None;
        for event in self.state.hockey.advance(dt) {
            match event {
                Event::Hit => {
                    if now - self.last_hit > 0.06 {
                        pulse = Some(Pulse::Tap);
                        self.last_hit = now;
                    }
                }
                Event::Goal(_) => {
                    self.save();
                    pulse = Some(Pulse::Eat);
                }
                Event::Won(_) => {
                    self.save();
                    pulse = Some(Pulse::Won);
                }
            }
        }
        if running && self.state.hockey.phase == HPhase::Paused {
            self.save();
        }
        let phase = self.state.hockey.phase;
        let label = match phase {
            HPhase::Ready => "Primeiro a 7".into(),
            HPhase::Playing => "".into(),
            HPhase::Paused => "Pausado".into(),
            HPhase::Goal(p) => format!("Gol do jogador {}!", p + 1),
            HPhase::Finished(p) => format!("Jogador {} venceu!", p + 1),
        };
        self.status(ui, l.info, &label, self.state.hockey.scores.map(u32::from));
        hockey_court(ui, l, &self.state.hockey);
        let toggle = ui.icon_button(
            if phase == HPhase::Playing {
                Icon::Pause
            } else {
                Icon::Play
            },
            Rect::new(l.info.center().x - 22., l.info.bottom() - 48., 44., 44.),
            false,
        ) || keys.iter().any(|&(k, _, r)| k == KeyCode::Space && !r);
        if toggle {
            match phase {
                HPhase::Playing => {
                    self.state.hockey.pause();
                }
                HPhase::Finished(_) => self.state.hockey.rematch(),
                _ => {
                    self.state.hockey.play();
                }
            }
            self.last = Some(now);
            self.fingers = [None; 2];
            self.mouse = None;
            self.save();
        }
        if phase != HPhase::Playing && (b.h >= 450. || l.landscape) {
            let text = match phase {
                HPhase::Ready => "Cada pessoa controla um lado",
                HPhase::Paused => "Continuar",
                HPhase::Goal(_) => "Próximo saque",
                HPhase::Finished(_) => "Revanche",
                _ => "",
            };
            fit(
                ui,
                text,
                Rect::new(l.info.x, l.info.bottom() - 85., l.info.w, 28.),
                13.,
                ui.theme.muted,
                false,
            );
        }
        pulse
    }
    fn hockey_input(
        &mut self,
        l: HockeyLayout,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        dt: f32,
    ) {
        let touches = touches();
        let live: Vec<_> = touches
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .collect();
        for i in 0..2 {
            if let Some(id) = self.fingers[i]
                && !live.iter().any(|t| t.id == id)
            {
                self.fingers[i] = None;
            }
        }
        for t in &live {
            if !self.fingers.contains(&Some(t.id)) {
                let point = touch_point(t.position, screen_dpi_scale());
                if l.court.contains(point) {
                    let world = l.local(point);
                    let i = usize::from(world.y < ah::HEIGHT / 2.);
                    if self.fingers[i].is_none() {
                        self.fingers[i] = Some(t.id);
                    }
                }
            }
        }
        for i in 0..2 {
            if let Some(t) = live.iter().find(|t| Some(t.id) == self.fingers[i]) {
                self.state
                    .hockey
                    .move_to(i, l.local(touch_point(t.position, screen_dpi_scale())));
            }
        }
        if touches.is_empty() {
            if let Some(point) = press.filter(|p| l.court.contains(*p)) {
                self.mouse = Some(usize::from(l.local(point).y < ah::HEIGHT / 2.));
            }
            if is_mouse_button_released(MouseButton::Left) {
                self.mouse = None;
            }
            if let Some(i) = self.mouse
                && is_mouse_button_down(MouseButton::Left)
            {
                self.state
                    .hockey
                    .move_to(i, l.local(Vec2::from(mouse_position())));
            }
        }
        for (i, codes) in [
            [KeyCode::A, KeyCode::D, KeyCode::W, KeyCode::S],
            [KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down],
        ]
        .into_iter()
        .enumerate()
        {
            let direction = vec2(
                f32::from(is_key_down(codes[1]) as u8) - f32::from(is_key_down(codes[0]) as u8),
                f32::from(is_key_down(codes[3]) as u8) - f32::from(is_key_down(codes[2]) as u8),
            );
            let d = if l.landscape {
                vec2(direction.y, -direction.x)
            } else {
                direction
            };
            self.state
                .hockey
                .move_to(i, self.state.hockey.target(i) + d * dt.min(0.05) * 1.5);
        }
        // Preserve a quick key tap even when key-up arrived before this frame.
        for &(key, _, repeat) in keys {
            if repeat || is_key_down(key) {
                continue;
            }
            let (player, direction) = match key {
                KeyCode::A => (0, vec2(-1., 0.)),
                KeyCode::D => (0, vec2(1., 0.)),
                KeyCode::W => (0, vec2(0., -1.)),
                KeyCode::S => (0, vec2(0., 1.)),
                KeyCode::Left => (1, vec2(-1., 0.)),
                KeyCode::Right => (1, vec2(1., 0.)),
                KeyCode::Up => (1, vec2(0., -1.)),
                KeyCode::Down => (1, vec2(0., 1.)),
                _ => continue,
            };
            let d = if l.landscape {
                vec2(direction.y, -direction.x)
            } else {
                direction
            };
            self.state
                .hockey
                .move_to(player, self.state.hockey.target(player) + d * 0.035);
        }
    }
    fn naval_transition(&mut self) {
        self.anchor = None;
        self.own = false;
        self.selected = 0;
        self.pan = BoardPan::default();
        self.save();
    }
    fn draw_naval(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        b: Rect,
    ) -> Option<Pulse> {
        let phase = self.state.naval.phase;
        if let NPhase::Cover { player, resume } = phase {
            let landscape = b.w >= 480. && b.h < 450.;
            let info = if landscape {
                Rect::new(b.center().x, b.y, b.w / 2., b.h)
            } else {
                b
            };
            let r = Rect::new(
                if landscape {
                    b.x + b.w * 0.25 - 36.
                } else {
                    b.center().x - 36.
                },
                b.y + (b.h - 140.) * 0.4,
                72.,
                92.,
            );
            bordered(r, 20., BLUE, ui.theme.bg);
            draw_circle_lines(r.center().x, r.center().y - 6., 9., 2., BLUE);
            draw_line(
                r.center().x,
                r.center().y + 4.,
                r.center().x,
                r.center().y + 20.,
                4.,
                BLUE,
            );
            fit(
                ui,
                &format!("Passe ao jogador {}", player + 1),
                Rect::new(
                    info.x,
                    if landscape {
                        b.y + 22.
                    } else {
                        r.bottom() + 18.
                    },
                    info.w,
                    36.,
                ),
                23.,
                ui.theme.text,
                true,
            );
            fit(
                ui,
                if resume == Resume::Setup {
                    "Posicione sua frota em segredo"
                } else {
                    "Sua tela está escondida"
                },
                Rect::new(
                    info.x,
                    if landscape {
                        b.y + 66.
                    } else {
                        r.bottom() + 58.
                    },
                    info.w,
                    26.,
                ),
                13.,
                ui.theme.muted,
                false,
            );
            if primary(
                ui,
                "Abrir minha tela",
                Rect::new(info.x, b.bottom() - 48., info.w, 48.),
                true,
            ) {
                self.state.naval.reveal();
                self.naval_transition();
            }
            return None;
        }
        let setup = matches!(phase, NPhase::Setup(_));
        let end = matches!(phase, NPhase::End(_));
        let view = self.state.naval.view();
        let landscape = b.w >= 480. && b.h < 450.;
        let info = if landscape {
            Rect::new(b.right() - 180., b.y, 180., b.h)
        } else {
            b
        };
        let label = if let NPhase::End(p) = phase {
            format!("Jogador {} venceu!", p + 1)
        } else if matches!(phase, NPhase::Report(_)) {
            let (cell, result) = view.as_ref().unwrap().last.unwrap();
            format!(
                "{} · {}",
                coordinate(cell),
                match result {
                    ShotResult::Miss => "Água",
                    ShotResult::Hit => "Acertou!",
                    ShotResult::Sunk(_) => "Afundou!",
                }
            )
        } else {
            format!("Jogador {}", self.state.naval.player() + 1)
        };
        self.status(ui, info, &label, self.state.naval.scores);
        let vp = if landscape {
            Rect::new(b.x, b.y, b.w - 196., b.h)
        } else {
            Rect::new(b.x, b.y + 48., b.w, b.h - 112.)
        };
        let side = (vp.w - 24.).min(520.);
        let tabs = if setup || landscape { 0. } else { 52. };
        let tools = if setup && !landscape { 120. } else { 44. };
        let content = tabs + side + 24. + tools;
        let (origin, tap) = page_input(&mut self.pan, &mut self.blocked, press, vp, content);
        let old = ui.override_pointer(tap);
        if landscape {
            ui.override_pointer(old);
        }
        if !landscape {
            clip(Some(vp));
        }
        let mut controls = if landscape {
            Rect::new(info.x, info.y + 48., info.w, info.h - 112.)
        } else {
            Rect::new(b.x, origin.y + tabs + side + 28., b.w, tools)
        };
        if !setup {
            let tr = if landscape {
                Rect::new(info.x, info.y + 48., info.w, 44.)
            } else {
                Rect::new(b.x, origin.y, b.w, 44.)
            };
            let half = tr.w / 2.;
            for i in 0..2 {
                let active = if end {
                    self.end_player == i
                } else {
                    self.own == (i == 1)
                };
                let title = if end {
                    if i == 0 { "Jogador 1" } else { "Jogador 2" }
                } else if i == 0 {
                    "Alvo"
                } else {
                    "Minha frota"
                };
                if ui.tab(
                    title,
                    Rect::new(tr.x + i as f32 * half, tr.y, half, 44.),
                    active,
                ) {
                    if end {
                        self.end_player = i;
                    } else {
                        self.own = i == 1;
                    }
                    self.anchor = None;
                    self.revision += 1;
                }
            }
            if landscape {
                controls.y += 50.;
                controls.h -= 50.;
            }
        }
        if landscape {
            ui.override_pointer(old);
            clip(Some(vp));
        }
        let board = Rect::new(vp.center().x - side / 2., origin.y + tabs + 20., side, side);
        let tiles = if end {
            self.state.naval.revealed_board(self.end_player).unwrap()
        } else if setup || self.own {
            view.as_ref().unwrap().own.clone()
        } else {
            view.as_ref().unwrap().target.clone()
        };
        draw_naval_board(ui, board, &tiles, self.anchor);
        let pointer = if landscape { tap } else { None };
        let old_board = if landscape {
            Some(ui.override_pointer(pointer))
        } else {
            None
        };
        // One index computation prevents boundary taps from choosing two cells.
        if (setup || matches!(phase, NPhase::Turn(_)) && !self.own)
            && let Some(point) = tap.filter(|p| board.contains(*p))
        {
            let x = ((point.x - board.x) / board.w * 10.) as usize;
            let y = ((point.y - board.y) / board.h * 10.) as usize;
            if x < 10 && y < 10 {
                let cell = y * 10 + x;
                if setup || self.state.naval.can_fire(cell) {
                    self.anchor = Some(cell);
                    self.revision += 1;
                }
            }
        }
        if let Some(previous) = old_board {
            ui.override_pointer(previous);
        }
        if setup && let Some(cell) = self.anchor {
            let ship = Ship {
                x: (cell % 10) as u8,
                y: (cell / 10) as u8,
                vertical: self.vertical,
            };
            let valid = self.state.naval.can_place(self.selected, ship);
            let unit = board.w / 10.;
            for n in 0..bs::LENGTHS[self.selected] {
                let x = cell % 10 + if self.vertical { 0 } else { n };
                let y = cell / 10 + if self.vertical { n } else { 0 };
                if x < 10 && y < 10 {
                    draw_rectangle_lines(
                        board.x + x as f32 * unit + 2.,
                        board.y + y as f32 * unit + 2.,
                        unit - 4.,
                        unit - 4.,
                        2.,
                        if valid { BLUE } else { CORAL },
                    );
                }
            }
        }
        if landscape {
            clip(None);
        } // The side controls do not scroll with the grid.
        if setup {
            let placements = *self.state.naval.placements().unwrap();
            let gap = 4.;
            let cw = (controls.w - 4. * gap) / 5.;
            for (i, length) in bs::LENGTHS.into_iter().enumerate() {
                let r = Rect::new(controls.x + i as f32 * (cw + gap), controls.y, cw, 44.);
                bordered(
                    r,
                    12.,
                    if i == self.selected {
                        BLUE
                    } else {
                        ui.theme.line
                    },
                    ui.theme.bg,
                );
                let u = ((cw - 10.) / 5.).min(9.);
                let start = r.center().x - length as f32 * u / 2.;
                for j in 0..length {
                    rounded(
                        Rect::new(start + j as f32 * u, r.center().y - 4., u - 1., 8.),
                        2.,
                        if placements[i].is_some() {
                            BLUE
                        } else {
                            ui.theme.muted
                        },
                    );
                }
                if ui.hit(r) {
                    self.selected = i;
                    self.anchor = None;
                    self.revision += 1;
                }
            }
            fit(
                ui,
                bs::NAMES[self.selected],
                Rect::new(controls.x, controls.y + 46., controls.w, 22.),
                12.,
                ui.theme.muted,
                false,
            );
            let half = (controls.w - 8.) / 2.;
            if ui.button(
                "↻",
                Rect::new(controls.x, controls.y + 72., half, 44.),
                false,
            ) || keys.iter().any(|&(k, _, r)| k == KeyCode::R && !r)
            {
                self.vertical = !self.vertical;
                self.revision += 1;
            }
            if dice_button(
                ui,
                Rect::new(controls.x + half + 8., controls.y + 72., half, 44.),
            ) || keys.iter().any(|&(k, _, r)| k == KeyCode::A && !r)
            {
                self.state
                    .naval
                    .automatic((get_time() * 1000000.) as u64 ^ self.revision);
                self.anchor = None;
                self.save();
            }
        } else if let Some(v) = &view {
            let message = if let Some((cell, result)) =
                v.last.filter(|_| matches!(phase, NPhase::Report(_)))
            {
                format!(
                    "{} · {}",
                    coordinate(cell),
                    match result {
                        ShotResult::Miss => "Água".into(),
                        ShotResult::Hit => "Acertou!".into(),
                        ShotResult::Sunk(i) => format!("{} afundado!", bs::NAMES[i]),
                    }
                )
            } else if let Some(cell) = self.anchor {
                format!("Alvo {}", coordinate(cell))
            } else {
                "Escolha uma coordenada".into()
            };
            fit(
                ui,
                &message,
                Rect::new(controls.x, controls.y, controls.w, 40.),
                14.,
                ui.theme.text,
                true,
            );
            for (i, sunk) in v.sunk.into_iter().enumerate() {
                let x = controls.center().x + (i as f32 - 2.) * 22.;
                draw_circle(
                    x,
                    controls.y + 46.,
                    4.,
                    if sunk { CORAL } else { ui.theme.line },
                );
            }
        }
        if !landscape {
            clip(None);
            ui.override_pointer(old);
        }
        let footer = Rect::new(info.x, info.bottom() - 48., info.w, 48.);
        if setup || matches!(phase, NPhase::Turn(_)) && !self.own {
            for &(key, _, repeat) in keys {
                if repeat {
                    continue;
                }
                let current = self.anchor.unwrap_or(0);
                let next = match key {
                    KeyCode::Left => Some(current.saturating_sub(1)),
                    KeyCode::Right => Some((current + 1).min(99)),
                    KeyCode::Up => Some(current.saturating_sub(10)),
                    KeyCode::Down => Some((current + 10).min(99)),
                    _ => None,
                };
                if let Some(cell) = next {
                    self.anchor = Some(cell);
                    let top = tabs + 20. + (cell / 10) as f32 * side / 10.;
                    self.pan.offset.y = self
                        .pan
                        .offset
                        .y
                        .min(top)
                        .max(top + side / 10. - vp.h)
                        .clamp(0., (content - vp.h).max(0.));
                    self.revision += 1;
                }
            }
        }
        let enter = !ui.keyboard_focus && keys.iter().any(|&(k, _, r)| k == KeyCode::Enter && !r);
        if setup {
            let candidate = self.anchor.map(|cell| Ship {
                x: (cell % 10) as u8,
                y: (cell / 10) as u8,
                vertical: self.vertical,
            });
            let seal = self.state.naval.ready() && candidate.is_none();
            let enabled =
                seal || candidate.is_some_and(|s| self.state.naval.can_place(self.selected, s));
            if primary(
                ui,
                if seal {
                    "Esconder e passar"
                } else {
                    "Posicionar"
                },
                footer,
                enabled,
            ) || enter && enabled
            {
                if seal {
                    self.state.naval.seal();
                    self.naval_transition();
                } else {
                    self.state.naval.place(self.selected, candidate.unwrap());
                    self.anchor = None;
                    self.selected = self
                        .state
                        .naval
                        .placements()
                        .unwrap()
                        .iter()
                        .position(Option::is_none)
                        .unwrap_or(self.selected);
                    self.save();
                }
            }
        } else if matches!(phase, NPhase::Turn(_)) {
            let enabled = !self.own && self.anchor.is_some_and(|c| self.state.naval.can_fire(c));
            if primary(ui, "Disparar", footer, enabled) || enter && enabled {
                let result = self.state.naval.fire(self.anchor.unwrap()).unwrap();
                self.anchor = None;
                self.save();
                return Some(match result {
                    ShotResult::Miss => Pulse::Tap,
                    ShotResult::Hit => Pulse::Eat,
                    ShotResult::Sunk(_) => Pulse::Won,
                });
            }
        } else if matches!(phase, NPhase::Report(_)) {
            if primary(ui, "Esconder e passar", footer, true) || enter {
                self.state.naval.pass();
                self.naval_transition();
            }
        } else if end && (primary(ui, "Revanche", footer, true) || enter) {
            self.state.naval.rematch();
            self.naval_transition();
        }
        None
    }
    fn rules(&self) -> Vec<&'static str> {
        if self.kind == Kind::Hockey {
            vec![
                "Duas pessoas jogam ao mesmo tempo. Jogador 1 é coral; jogador 2 é azul. Cada pessoa arrasta seu rebatedor no próprio lado da mesa. Dois dedos independentes podem jogar juntos.",
                "Empurre o disco para dentro do gol adversário. Cada gol vale um ponto; vence quem chegar a sete. Rebatedores não atravessam o meio da mesa.",
                "No computador, jogador 1 usa WASD e jogador 2 usa as setas. O mouse controla um rebatedor enquanto estiver pressionado. Space inicia ou pausa. Sair, abrir menus ou trocar de janela pausa a partida.",
            ]
        } else {
            vec![
                "Cada jogador posiciona cinco navios em segredo num tabuleiro 10×10: tamanhos 5, 4, 3, 3 e 2. Não podem se sobrepor nem sair do tabuleiro; podem encostar.",
                "Escolha um navio, toque na primeira casa e confirme Posicionar. ↻ gira o navio; o dado monta uma frota aleatória. Você pode reposicionar navios já colocados. R gira; A organiza aleatoriamente. Quando terminar, esconda e passe o aparelho.",
                "Na sua vez, escolha uma casa do tabuleiro Alvo e confirme Disparar. Água significa erro; coral significa acerto; um navio completo atingido fica marcado como afundado. Minha frota mostra seus próprios navios e os tiros recebidos.",
                "Alternem um disparo por turno, mesmo quando acertarem. Não é possível atirar de novo na mesma casa. Vence quem afundar os cinco navios adversários. O placar fica salvo e a revanche alterna quem começa.",
                "Passe o aparelho somente após esconder a tela. Menus, voltar, recarregar e interrupções também cobrem as informações privadas. Deslizar rola o tabuleiro e nunca dispara. Setas escolhem casas; Enter confirma; Tab navega entre opções.",
            ]
        }
    }
    fn draw_rules(&mut self, ui: &mut Ui, press: Option<Vec2>, b: Rect) {
        let rules = self.rules();
        let vp = Rect::new(b.x, b.y, b.w, b.h - 64.);
        let heights: Vec<_> = rules
            .iter()
            .map(|r| {
                crate::online_view::wrapped_lines(ui, r, b.w - 24., 16.).len() as f32 * 25. + 24.
            })
            .collect();
        let (o, _) = page_input(
            &mut self.pan,
            &mut self.blocked,
            press,
            vp,
            heights.iter().sum(),
        );
        let mut y = o.y;
        clip(Some(vp));
        for (r, h) in rules.iter().zip(heights) {
            wrap(
                ui,
                r,
                Rect::new(b.x + 12., y, b.w - 24., h),
                16.,
                ui.theme.text,
                false,
            );
            y += h;
        }
        clip(None);
        if primary(
            ui,
            "Fechar",
            Rect::new(b.x, b.bottom() - 48., b.w, 48.),
            true,
        ) {
            self.help = false;
            self.pan = BoardPan::default();
            self.revision += 1;
        }
    }
}
fn coordinate(cell: usize) -> String {
    format!("{}{}", (b'A' + (cell % 10) as u8) as char, cell / 10 + 1)
}
#[derive(Clone, Copy)]
struct HockeyLayout {
    court: Rect,
    info: Rect,
    landscape: bool,
}
impl HockeyLayout {
    fn point(self, p: Vec2) -> Vec2 {
        if self.landscape {
            vec2(
                self.court.x + (ah::HEIGHT - p.y) * self.court.w / ah::HEIGHT,
                self.court.y + p.x * self.court.h,
            )
        } else {
            self.court.point() + vec2(p.x * self.court.w, p.y * self.court.h / ah::HEIGHT)
        }
    }
    fn local(self, p: Vec2) -> Vec2 {
        if self.landscape {
            vec2(
                (p.y - self.court.y) / self.court.h,
                ah::HEIGHT - (p.x - self.court.x) / self.court.w * ah::HEIGHT,
            )
        } else {
            vec2(
                (p.x - self.court.x) / self.court.w,
                (p.y - self.court.y) / self.court.h * ah::HEIGHT,
            )
        }
    }
    fn scale(self) -> f32 {
        if self.landscape {
            self.court.h
        } else {
            self.court.w
        }
    }
}
fn hockey_layout(b: Rect) -> HockeyLayout {
    let landscape = b.w >= 480. && b.h < 450.;
    let info = if landscape {
        Rect::new(b.right() - 188., b.y, 188., b.h)
    } else {
        b
    };
    let field = if landscape {
        Rect::new(b.x, b.y, b.w - 204., b.h)
    } else {
        Rect::new(
            b.x,
            b.y + 48.,
            b.w,
            b.h - if b.h < 450. { 104. } else { 150. },
        )
    };
    let scale = if landscape {
        (field.w / ah::HEIGHT).min(field.h)
    } else {
        field.w.min(field.h / ah::HEIGHT)
    }
    .min(460.);
    let size = if landscape {
        vec2(scale * ah::HEIGHT, scale)
    } else {
        vec2(scale, scale * ah::HEIGHT)
    };
    HockeyLayout {
        court: Rect::new(
            field.center().x - size.x / 2.,
            field.center().y - size.y / 2.,
            size.x,
            size.y,
        ),
        info,
        landscape,
    }
}
fn hockey_court(ui: &Ui, l: HockeyLayout, g: &Hockey) {
    let r = l.court;
    let s = l.scale();
    let line = if ui.theme.saver {
        color_u8!(38, 48, 43, 255)
    } else {
        color_u8!(216, 231, 237, 255)
    };
    bordered(
        r,
        24.,
        line,
        if ui.theme.saver {
            BLACK
        } else {
            color_u8!(247, 251, 253, 255)
        },
    );
    let a = l.point(vec2(0., ah::HEIGHT / 2.));
    let b = l.point(vec2(1., ah::HEIGHT / 2.));
    draw_line(a.x, a.y, b.x, b.y, 1.5, line);
    let c = l.point(vec2(0.5, ah::HEIGHT / 2.));
    draw_circle_lines(c.x, c.y, s * 0.19, 1.5, line);
    draw_circle(c.x, c.y, 3., line);
    for (i, y) in [ah::HEIGHT, 0.].into_iter().enumerate() {
        let a = l.point(vec2(0.3, y));
        let b = l.point(vec2(0.7, y));
        draw_line(a.x, a.y, b.x, b.y, 7., if i == 0 { CORAL } else { BLUE });
        let p = l.point(vec2(0.5, if i == 0 { ah::HEIGHT - 0.19 } else { 0.19 }));
        draw_circle_lines(p.x, p.y, s * 0.17, 1.5, line);
    }
    for (i, color) in [CORAL, BLUE].into_iter().enumerate() {
        let p = l.point(g.render_paddle(i));
        if !ui.theme.saver {
            draw_circle(p.x, p.y + 3., s * ah::MALLET, color_u8!(0, 0, 0, 12));
        }
        draw_circle(p.x, p.y, s * ah::MALLET, color);
        draw_circle_lines(
            p.x,
            p.y,
            s * ah::MALLET * 0.7,
            2.,
            Color::new(1., 1., 1., 0.6),
        );
        draw_circle(
            p.x,
            p.y,
            s * ah::MALLET * 0.24,
            color_u8!(255, 255, 255, 190),
        );
    }
    let p = l.point(g.render_puck());
    draw_circle(p.x, p.y, s * ah::PUCK, ui.theme.text);
    draw_circle_lines(p.x, p.y, s * ah::PUCK * 0.58, 1., ui.theme.bg);
}
fn draw_naval_board(ui: &Ui, r: Rect, tiles: &[Tile], selected: Option<usize>) {
    let u = r.w / 10.;
    for i in 0..10 {
        fit(
            ui,
            &((b'A' + i as u8) as char).to_string(),
            Rect::new(r.x + i as f32 * u, r.y - 20., u, 16.),
            10.,
            ui.theme.muted,
            false,
        );
    }
    for (c, t) in tiles.iter().enumerate() {
        let cell = Rect::new(r.x + (c % 10) as f32 * u, r.y + (c / 10) as f32 * u, u, u);
        let fill = match t {
            Tile::Ship => {
                if ui.theme.saver {
                    color_u8!(10, 29, 42, 255)
                } else {
                    color_u8!(215, 234, 244, 255)
                }
            }
            Tile::Sunk => {
                if ui.theme.saver {
                    color_u8!(35, 15, 15, 255)
                } else {
                    color_u8!(253, 229, 225, 255)
                }
            }
            _ => ui.theme.bg,
        };
        bordered(
            Rect::new(cell.x + 1., cell.y + 1., u - 2., u - 2.),
            4.,
            ui.theme.line,
            fill,
        );
        if *t == Tile::Miss {
            draw_circle(cell.center().x, cell.center().y, u * 0.075, ui.theme.muted);
        } else if matches!(t, Tile::Hit | Tile::Sunk) {
            let p = cell.center();
            let d = u * 0.18;
            draw_line(p.x - d, p.y - d, p.x + d, p.y + d, 2., CORAL);
            draw_line(p.x + d, p.y - d, p.x - d, p.y + d, 2., CORAL);
        }
        if selected == Some(c) {
            draw_rectangle_lines(cell.x + 2., cell.y + 2., u - 4., u - 4., 2., BLUE);
            let p = cell.center();
            draw_circle_lines(p.x, p.y, u * 0.26, 1.5, BLUE);
        }
        if c % 10 == 0 {
            fit(
                ui,
                &(c / 10 + 1).to_string(),
                Rect::new(r.x - 13., cell.y, 12., u),
                9.,
                ui.theme.muted,
                false,
            );
        }
    }
}
fn dice_button(ui: &mut Ui, r: Rect) -> bool {
    bordered(r, 17., ui.theme.line, ui.theme.bg);
    let c = r.center();
    draw_rectangle_lines(c.x - 10., c.y - 10., 20., 20., 1.5, ui.theme.text);
    for p in [
        vec2(-5., -5.),
        vec2(5., -5.),
        vec2(0., 0.),
        vec2(-5., 5.),
        vec2(5., 5.),
    ] {
        draw_circle(c.x + p.x, c.y + p.y, 1.8, ui.theme.text);
    }
    ui.hit(r)
}
pub fn preview(ui: &Ui, kind: Kind, r: Rect) {
    if kind == Kind::Hockey {
        let size = vec2((r.h - 10.) / ah::HEIGHT, r.h - 10.);
        let l = HockeyLayout {
            court: Rect::new(r.center().x - size.x / 2., r.y + 5., size.x, size.y),
            info: r,
            landscape: false,
        };
        let g = Hockey::default();
        hockey_court(ui, l, &g);
    } else {
        let side = (r.h - 22.).min(r.w - 24.);
        let b = Rect::new(r.center().x - side / 2., r.y + 16., side, side);
        let mut tiles = vec![Tile::Water; 100];
        for c in [22, 23, 24, 25, 54, 64, 74] {
            tiles[c] = Tile::Ship;
        }
        tiles[22] = Tile::Hit;
        tiles[54] = Tile::Sunk;
        tiles[32] = Tile::Miss;
        draw_naval_board(ui, b, &tiles, None);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hockey_layout_maps_both_orientations_and_fits_touch_screens() {
        for (w, h) in [
            (280., 360.),
            (390., 844.),
            (568., 320.),
            (820., 1180.),
            (1440., 900.),
        ] {
            let b = Rect::new(16., 66., w - 32., h - 82.);
            let l = hockey_layout(b);
            assert!(
                l.court.x >= b.x - 0.1
                    && l.court.right() <= b.right() + 0.1
                    && l.court.bottom() <= b.bottom() + 0.1
            );
            for p in [vec2(0., 0.), vec2(1., ah::HEIGHT), vec2(0.3, 1.2)] {
                assert!(l.local(l.point(p)).distance(p) < 0.0001);
            }
        }
    }
}
