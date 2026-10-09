//! Responsive vectors and private shared-device handoffs for local two-player games.
use crate::{
    online_style::primary,
    online_view::{clip, fit, page_input, wrap},
    platform,
    ui::{CORAL, Icon, Ui, bordered, rounded},
};
use jarcade::{
    board_pan::BoardPan,
    feedback::Pulse,
    local_pair::{
        FACE_COUNT, FACE_NAMES, FacePhase, Feedback, Grid, Kind, MmPhase, Outcome, State,
    },
};
use macroquad::{miniquad::KeyMods, prelude::*};
const BLUE: Color = color_u8!(61, 117, 198, 255);
const VIOLET: Color = color_u8!(122, 88, 183, 255);
const PEGS: [Color; 6] = [
    color_u8!(222, 86, 76, 255),
    BLUE,
    color_u8!(39, 143, 113, 255),
    VIOLET,
    color_u8!(190, 132, 28, 255),
    color_u8!(191, 71, 132, 255),
];
pub struct PairPage {
    state: State,
    kind: Kind,
    pub revision: u64,
    pan: BoardPan,
    blocked: bool,
    size: Vec2,
    help: bool,
    reset: bool,
    selected: usize,
    guessing: bool,
    guess: Option<usize>,
    save_failed: bool,
}
impl PairPage {
    pub fn new() -> Self {
        Self {
            state: platform::load_pair(),
            kind: Kind::TicTacToe,
            revision: 0,
            pan: BoardPan::default(),
            blocked: false,
            size: Vec2::ZERO,
            help: false,
            reset: false,
            selected: 0,
            guessing: false,
            guess: None,
            save_failed: false,
        }
    }
    fn save(&mut self) {
        self.revision += 1;
        self.save_failed = !platform::save_pair(&self.state);
    }
    pub fn enter(&mut self, kind: Kind) {
        self.interrupt();
        self.kind = kind;
        self.help = false;
        self.reset = false;
        self.pan = BoardPan::default();
        self.selected = 0;
    }
    pub fn interrupt(&mut self) {
        let changed = self.state.conceal();
        self.pan.cancel();
        self.blocked = true;
        self.guess = None;
        self.guessing = false;
        if changed {
            self.save();
        } else {
            self.revision += 1;
        }
    }
    pub fn needs_frame(&self) -> bool {
        self.pan.active()
    }
    pub fn back(&mut self) -> bool {
        self.revision += 1;
        if self.help || self.reset || self.guess.is_some() {
            self.help = false;
            self.reset = false;
            self.guess = None;
            self.pan = BoardPan::default();
            false
        } else {
            self.interrupt();
            true
        }
    }
    fn transition(&mut self) {
        self.pan = BoardPan::default();
        self.selected = 0;
        self.guess = None;
        self.guessing = false;
        self.save();
    }
    fn grid(&self) -> &Grid {
        if self.kind == Kind::TicTacToe {
            &self.state.tic
        } else {
            &self.state.connect
        }
    }
    fn grid_mut(&mut self) -> &mut Grid {
        if self.kind == Kind::TicTacToe {
            &mut self.state.tic
        } else {
            &mut self.state.connect
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
            return s + "Confirmar reinício do placar e da partida. Cancelar ou Reiniciar.";
        }
        match self.kind {
            Kind::TicTacToe | Kind::ConnectFour => {
                let g = self.grid();
                let p = g.position();
                s.push_str(&format!(
                    "Rodada {}. Placar {} a {}. {:?}. Jogador {}. Tabuleiro: {:?}. ",
                    g.round(),
                    g.scores()[0],
                    g.scores()[1],
                    p.outcome,
                    p.turn + 1,
                    p.cells
                ));
            }
            Kind::Mastermind => {
                let g = &self.state.mastermind;
                s.push_str(&format!(
                    "{:?}. Jogador {}. Rodada {}. Placar {} a {}. ",
                    g.phase(),
                    g.player() + 1,
                    g.round(),
                    g.scores()[0],
                    g.scores()[1]
                ));
                if matches!(g.phase(), MmPhase::CodeCover | MmPhase::GuessCover) {
                    s.push_str("Código oculto. Passe o aparelho e abra sua tela.");
                } else {
                    s.push_str(&format!("Cores selecionadas: {:?}. ", g.slots()));
                    for (i, (guess, f)) in g.history().iter().enumerate() {
                        s.push_str(&format!(
                            "Tentativa {}: {:?}, {} exatas e {} em outra posição. ",
                            i + 1,
                            guess,
                            f.exact,
                            f.colour
                        ));
                    }
                    if g.phase() == MmPhase::Result {
                        s.push_str(if g.solved() {
                            "Código descoberto."
                        } else {
                            "Tentativas esgotadas."
                        });
                    }
                }
            }
            Kind::Faces => {
                let g = &self.state.faces;
                s.push_str(&format!(
                    "{:?}. Rodada {}. Placar {} a {}. ",
                    g.phase(),
                    g.round(),
                    g.scores()[0],
                    g.scores()[1]
                ));
                if let Some(id) = g.own() {
                    s.push_str(&format!("Meu personagem: {}. ", FACE_NAMES[id]));
                }
                if let Some(ids) = g.result() {
                    s.push_str(&format!(
                        "Identidades reveladas: {} e {}. ",
                        FACE_NAMES[ids[0]], FACE_NAMES[ids[1]]
                    ));
                }
                if matches!(g.phase(), FacePhase::Cover { .. }) {
                    s.push_str("Identidades ocultas. Passe o aparelho.");
                } else if matches!(g.phase(), FacePhase::Choose { .. } | FacePhase::Turn { .. }) {
                    for (i, &name) in FACE_NAMES.iter().enumerate() {
                        s.push_str(&format!(
                            "{}{}; ",
                            name,
                            if g.marks()[i] { " eliminado" } else { "" }
                        ));
                    }
                    s.push_str(if self.guessing {
                        "Modo palpite: escolha um personagem e confirme."
                    } else {
                        "Toque para marcar ou restaurar personagens. Passar ou Palpite."
                    });
                }
            }
        }
        if let Some(id) = self.guess {
            s.push_str(&format!(
                "Confirmar palpite: {}? Um erro perde a rodada.",
                FACE_NAMES[id]
            ));
        }
        if self.save_failed {
            s.push_str("Não foi possível salvar o progresso.");
        }
        s
    }
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
    ) -> (bool, Option<Pulse>) {
        let size = vec2(screen_width(), screen_height());
        if size != self.size {
            self.interrupt();
            self.size = size;
        }
        ui.theme.accent = match self.kind {
            Kind::Faces | Kind::TicTacToe => CORAL,
            Kind::Mastermind => VIOLET,
            Kind::ConnectFour => BLUE,
        };
        if ui.theme.saver {
            ui.theme.accent = Color::new(
                (ui.theme.accent.r + 0.3).min(1.),
                (ui.theme.accent.g + 0.3).min(1.),
                (ui.theme.accent.b + 0.3).min(1.),
                1.,
            );
        }
        let w = (size.x - 32.).min(900.);
        let x = (size.x - w) / 2.;
        let b = Rect::new(x, 66., w, size.y - 82.);
        if ui.icon_button(Icon::Back, Rect::new(x, 8., 44., 44.), false) && self.back() {
            return (true, None);
        }
        fit(
            ui,
            self.kind.title(),
            Rect::new(x + 50., 8., w - 150., 44.),
            22.,
            ui.theme.text,
            true,
        );
        if ui.button("↻", Rect::new(b.right() - 94., 8., 44., 44.), false) {
            self.interrupt();
            self.reset = true;
            self.help = false;
            self.revision += 1;
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
        if self.reset || self.guess.is_some() {
            let pulse = self.modal(ui, b);
            return (false, pulse);
        }
        let pulse = match self.kind {
            Kind::TicTacToe | Kind::ConnectFour => self.draw_grid(ui, press, keys, b),
            Kind::Mastermind => self.draw_mm(ui, press, keys, b),
            Kind::Faces => self.draw_faces(ui, press, b),
        };
        if self.save_failed {
            fit(
                ui,
                "Progresso não salvo",
                Rect::new(b.x, 54., b.w, 15.),
                10.,
                CORAL,
                false,
            );
        }
        (false, pulse)
    }
    fn status(&self, ui: &Ui, b: Rect, label: &str, scores: [u32; 2]) {
        fit(
            ui,
            label,
            Rect::new(b.x, b.y, b.w - 112., 36.),
            16.,
            ui.theme.text,
            true,
        );
        bordered(
            Rect::new(b.right() - 104., b.y, 104., 36.),
            18.,
            ui.theme.line,
            ui.theme.bg,
        );
        fit(
            ui,
            &format!("{}  ·  {}", scores[0], scores[1]),
            Rect::new(b.right() - 96., b.y, 88., 36.),
            17.,
            ui.theme.text,
            true,
        );
    }
    fn cover(&mut self, ui: &mut Ui, b: Rect, player: u8, subtitle: &str) -> bool {
        let landscape = b.w >= 480. && b.h < 450.;
        let center = if landscape {
            vec2(b.x + b.w * 0.25, b.center().y)
        } else {
            vec2(b.center().x, b.y + (b.h - 70.) * 0.42)
        };
        let width = if landscape { b.w * 0.46 } else { b.w };
        let right = if landscape {
            Rect::new(b.center().x, b.y, width, b.h)
        } else {
            b
        };
        let s = (b.h * 0.2).clamp(28., 58.);
        bordered(
            Rect::new(center.x - s * 0.72, center.y - s, s * 1.44, s * 2.),
            16.,
            ui.theme.accent,
            ui.theme.bg,
        );
        draw_circle_lines(center.x, center.y - 3., s * 0.17, 2., ui.theme.accent);
        draw_line(
            center.x,
            center.y + s * 0.08,
            center.x,
            center.y + s * 0.27,
            3.,
            ui.theme.accent,
        );
        let text_y = if landscape {
            b.y + 38.
        } else {
            center.y + s + 24.
        };
        fit(
            ui,
            &format!("Passe ao jogador {}", player + 1),
            Rect::new(right.x, text_y, right.w, 32.),
            23.,
            ui.theme.text,
            true,
        );
        fit(
            ui,
            subtitle,
            Rect::new(right.x, text_y + 38., right.w, 26.),
            13.,
            ui.theme.muted,
            false,
        );
        primary(
            ui,
            "Abrir minha tela",
            Rect::new(right.x, b.bottom() - 48., right.w, 48.),
            true,
        )
    }
    fn draw_grid(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        b: Rect,
    ) -> Option<Pulse> {
        let p = self.grid().position();
        let scores = self.grid().scores();
        let label = match p.outcome {
            Outcome::Playing => format!("Jogador {}", p.turn + 1),
            Outcome::Won(n) => format!("Jogador {} venceu!", n + 1),
            Outcome::Draw => "Empate!".into(),
        };
        let l = grid_layout(self.kind, b);
        self.status(ui, l.info, &label, scores);
        let (_, tap) = page_input(&mut self.pan, &mut self.blocked, press, l.board, l.board.h);
        let old = ui.override_pointer(tap);
        let mut moved = false;
        let (cols, rows) = self.grid().size();
        let cell = l.board.w / cols as f32;
        if self.kind == Kind::ConnectFour {
            bordered(
                l.board,
                20.,
                ui.theme.accent,
                if ui.theme.saver {
                    BLACK
                } else {
                    color_u8!(235, 243, 253, 255)
                },
            );
        }
        for y in 0..rows {
            for x in 0..cols {
                let r = Rect::new(
                    l.board.x + x as f32 * cell,
                    l.board.y + y as f32 * cell,
                    cell,
                    cell,
                );
                let i = y * cols + x;
                let win = p.line.contains(&i);
                if self.kind == Kind::TicTacToe {
                    if x > 0 {
                        draw_line(r.x, l.board.y, r.x, l.board.bottom(), 2., ui.theme.line);
                    }
                    if y > 0 {
                        draw_line(l.board.x, r.y, l.board.right(), r.y, 2., ui.theme.line);
                    }
                    if win {
                        rounded(
                            Rect::new(r.x + 5., r.y + 5., r.w - 10., r.h - 10.),
                            16.,
                            if ui.theme.saver {
                                BLACK
                            } else {
                                color_u8!(255, 239, 231, 255)
                            },
                        );
                    }
                    symbol(
                        r.center(),
                        cell * 0.23,
                        p.cells[i],
                        if p.cells[i] == 1 {
                            ui.theme.accent
                        } else {
                            BLUE
                        },
                    );
                    if p.outcome == Outcome::Playing
                        && ui.hit(r)
                        && !moved
                        && self.grid_mut().play(i)
                    {
                        moved = true;
                        self.save();
                    }
                } else {
                    let radius = cell * 0.36;
                    draw_circle(r.center().x, r.center().y, radius, ui.theme.bg);
                    if p.cells[i] != 0 {
                        draw_circle(
                            r.center().x,
                            r.center().y,
                            radius,
                            if p.cells[i] == 1 { CORAL } else { BLUE },
                        );
                        draw_circle_lines(
                            r.center().x,
                            r.center().y,
                            radius - 4.,
                            2.,
                            if p.cells[i] == 1 {
                                color_u8!(255, 183, 152, 255)
                            } else {
                                color_u8!(150, 190, 244, 255)
                            },
                        );
                    }
                    if win {
                        draw_circle_lines(
                            r.center().x,
                            r.center().y,
                            radius + 3.,
                            2.,
                            ui.theme.text,
                        );
                    }
                }
            }
        }
        if self.kind == Kind::ConnectFour && p.outcome == Outcome::Playing {
            for x in 0..cols {
                if ui.hit(Rect::new(
                    l.board.x + x as f32 * cell,
                    l.board.y,
                    cell,
                    l.board.h,
                )) && !moved
                    && self.grid_mut().play(x)
                {
                    moved = true;
                    self.save();
                }
            }
        }
        ui.override_pointer(old);
        for &(key, _, repeat) in keys {
            if repeat || p.outcome != Outcome::Playing {
                continue;
            }
            let n = match key {
                KeyCode::Key1 => Some(0),
                KeyCode::Key2 => Some(1),
                KeyCode::Key3 => Some(2),
                KeyCode::Key4 => Some(3),
                KeyCode::Key5 => Some(4),
                KeyCode::Key6 => Some(5),
                KeyCode::Key7 => Some(6),
                KeyCode::Key8 => Some(7),
                KeyCode::Key9 => Some(8),
                _ => None,
            };
            if let Some(n) = n
                && self.grid_mut().play(n)
            {
                self.save();
            }
        }
        let now = self.grid().position().outcome;
        let pulse = (now != p.outcome && matches!(now, Outcome::Won(_))).then_some(Pulse::Won);
        if p.outcome != Outcome::Playing {
            if primary(ui, "Revanche", l.footer, true) {
                self.grid_mut().next_round();
                self.transition();
            }
        } else {
            fit(
                ui,
                if self.kind == Kind::TicTacToe {
                    "X  ·  O"
                } else {
                    "4 peças em linha"
                },
                l.footer,
                14.,
                ui.theme.muted,
                false,
            );
        }
        pulse
    }
    fn draw_mm(
        &mut self,
        ui: &mut Ui,
        press: Option<Vec2>,
        keys: &[(KeyCode, KeyMods, bool)],
        b: Rect,
    ) -> Option<Pulse> {
        let phase = self.state.mastermind.phase();
        if matches!(phase, MmPhase::CodeCover | MmPhase::GuessCover) {
            let player = self.state.mastermind.player();
            if self.cover(
                ui,
                b,
                player,
                if phase == MmPhase::CodeCover {
                    "Crie um código em segredo"
                } else {
                    "Descubra o código oculto"
                },
            ) {
                self.state.mastermind.reveal();
                self.transition();
            }
            return None;
        }
        let landscape = b.w >= 480. && b.h < 450.;
        let info = if landscape {
            Rect::new(b.right() - 230., b.y, 230., b.h)
        } else {
            b
        };
        let label = if phase == MmPhase::Code {
            "Crie seu código".into()
        } else if phase == MmPhase::Result {
            if self.state.mastermind.solved() {
                "Código descoberto!".into()
            } else {
                "Código resistiu!".into()
            }
        } else {
            format!("Tentativa {}/10", self.state.mastermind.history().len() + 1)
        };
        self.status(ui, info, &label, self.state.mastermind.scores());
        let history = self.state.mastermind.history();
        let small = !landscape && b.h < 330. && phase != MmPhase::Code;
        let small_vp = Rect::new(b.x, b.y + 48., b.w, b.h - 112.);
        let mut small_origin = b.y;
        let mut small_pointer = None;
        if small {
            let (o, tap) = page_input(
                &mut self.pan,
                &mut self.blocked,
                press,
                small_vp,
                history.len() as f32 * 58. + 170.,
            );
            small_origin = o.y;
            small_pointer = Some(ui.override_pointer(tap));
        }
        if phase != MmPhase::Code {
            let vp = if small {
                small_vp
            } else if landscape {
                Rect::new(b.x, b.y, b.w - 246., b.h)
            } else {
                Rect::new(b.x, b.y + 48., b.w, (b.h - 258.).max(48.))
            };
            let rh = 58.;
            let o = if small {
                vec2(b.x, small_origin)
            } else {
                page_input(
                    &mut self.pan,
                    &mut self.blocked,
                    press,
                    vp,
                    history.len() as f32 * rh,
                )
                .0
            };
            clip(Some(vp));
            for (i, (guess, f)) in history.iter().enumerate() {
                let r = Rect::new(vp.x, o.y + i as f32 * rh, vp.w, rh - 6.);
                bordered(r, 12., ui.theme.line, ui.theme.bg);
                fit(
                    ui,
                    &(i + 1).to_string(),
                    Rect::new(r.x + 2., r.y, 24., r.h),
                    12.,
                    ui.theme.muted,
                    false,
                );
                let step = ((r.w - 112.) / 4.).clamp(24., 42.);
                for (j, &v) in guess.iter().enumerate() {
                    peg(
                        ui,
                        vec2(r.x + 34. + j as f32 * step + step / 2., r.center().y),
                        step.min(38.) / 2. - 2.,
                        Some(v),
                        false,
                    );
                }
                draw_feedback(ui, vec2(r.right() - 40., r.center().y), *f);
            }
            clip(None);
        }
        if small {
            clip(Some(small_vp));
        }
        let row = if small {
            Rect::new(
                b.x,
                small_origin + history.len() as f32 * 58. + 8.,
                b.w,
                54.,
            )
        } else if phase == MmPhase::Code {
            Rect::new(info.x, info.y + (info.h - 140.) * 0.45, info.w, 54.)
        } else {
            Rect::new(info.x, info.bottom() - 200., info.w, 54.)
        };
        let slots = self.state.mastermind.slots();
        let step = (row.w / 4.).min(72.);
        let start = row.center().x - step * 2.;
        for (i, v) in slots.into_iter().enumerate() {
            let r = Rect::new(start + i as f32 * step, row.y, step, 54.);
            peg(
                ui,
                r.center(),
                22.,
                v,
                self.selected == i && phase != MmPhase::Result,
            );
            if phase != MmPhase::Result && ui.hit(r) {
                self.selected = i;
                self.revision += 1;
            }
        }
        if phase == MmPhase::Result {
            fit(
                ui,
                "Código revelado",
                Rect::new(info.x, row.y + 58., info.w, 24.),
                12.,
                ui.theme.muted,
                false,
            );
            if let Some(old) = small_pointer {
                clip(None);
                ui.override_pointer(old);
            }
            if primary(
                ui,
                "Trocar papéis",
                Rect::new(info.x, info.bottom() - 48., info.w, 48.),
                true,
            ) {
                self.state.mastermind.next_round();
                self.transition();
            }
            return None;
        }
        let palette_y = if small || phase == MmPhase::Code {
            row.bottom() + 16.
        } else {
            info.bottom() - 134.
        };
        let gap = 4.;
        let pw = ((info.w - gap * 5.) / 6.).min(52.);
        let px = info.center().x - (6. * pw + 5. * gap) / 2.;
        for i in 0..6 {
            let r = Rect::new(px + i as f32 * (pw + gap), palette_y, pw, 44.);
            peg(ui, r.center(), pw.min(42.) / 2. - 2., Some(i as u8), false);
            if ui.hit(r) {
                self.put_colour(i as u8);
            }
        }
        for &(key, _, repeat) in keys {
            if repeat {
                continue;
            }
            let value = match key {
                KeyCode::Key1 => Some(0),
                KeyCode::Key2 => Some(1),
                KeyCode::Key3 => Some(2),
                KeyCode::Key4 => Some(3),
                KeyCode::Key5 => Some(4),
                KeyCode::Key6 => Some(5),
                _ => None,
            };
            if let Some(v) = value {
                self.put_colour(v);
            } else if matches!(key, KeyCode::Backspace | KeyCode::Delete) {
                if self.state.mastermind.set(self.selected, None) {
                    self.save();
                }
            } else if key == KeyCode::Left {
                self.selected = (self.selected + 3) % 4;
                self.revision += 1;
            } else if key == KeyCode::Right {
                self.selected = (self.selected + 1) % 4;
                self.revision += 1;
            }
        }
        if let Some(old) = small_pointer {
            clip(None);
            ui.override_pointer(old);
        }
        let footer = Rect::new(info.x, info.bottom() - 48., info.w, 48.);
        if primary(
            ui,
            if phase == MmPhase::Code {
                "Esconder e passar"
            } else {
                "Testar código"
            },
            footer,
            self.state.mastermind.ready(),
        ) || !ui.keyboard_focus
            && keys.iter().any(|&(k, _, r)| k == KeyCode::Enter && !r)
            && self.state.mastermind.ready()
        {
            if phase == MmPhase::Code {
                self.state.mastermind.seal();
                self.transition();
            } else {
                self.state.mastermind.submit();
                self.selected = 0;
                self.save();
                let vp_h = if landscape {
                    b.h
                } else {
                    (b.h - 258.).max(48.)
                };
                self.pan.offset.y = if small {
                    (self.state.mastermind.history().len() as f32 * 58. + 170. - small_vp.h).max(0.)
                } else {
                    (self.state.mastermind.history().len() as f32 * 58. - vp_h).max(0.)
                };
                if self.state.mastermind.phase() == MmPhase::Result {
                    return Some(if self.state.mastermind.solved() {
                        Pulse::Won
                    } else {
                        Pulse::Lost
                    });
                }
            }
        }
        None
    }
    fn put_colour(&mut self, v: u8) {
        if !matches!(
            self.state.mastermind.phase(),
            MmPhase::Code | MmPhase::Guess
        ) {
            return;
        }
        let changed = self.state.mastermind.set(self.selected, Some(v));
        self.selected = (self.selected + 1) % 4;
        if changed {
            self.save();
        } else {
            self.revision += 1;
        }
    }
    fn draw_faces(&mut self, ui: &mut Ui, press: Option<Vec2>, b: Rect) -> Option<Pulse> {
        let phase = self.state.faces.phase();
        if let FacePhase::Cover { player, choosing } = phase {
            if self.cover(
                ui,
                b,
                player,
                if choosing {
                    "Escolha seu personagem em segredo"
                } else {
                    "Só você deve olhar esta tela"
                },
            ) {
                self.state.faces.reveal();
                self.transition();
            }
            return None;
        }
        if let FacePhase::End { winner } = phase {
            self.status(
                ui,
                b,
                &format!("Jogador {} venceu!", winner + 1),
                self.state.faces.scores(),
            );
            let ids = self.state.faces.result().unwrap();
            let size = (b.h - 150.).min(b.w * 0.42).clamp(70., 180.);
            for (i, id) in ids.into_iter().enumerate() {
                let r = Rect::new(
                    b.center().x + (i as f32 - 1.) * (size + 12.) + 6.,
                    b.y + 62.,
                    size,
                    size * 1.1,
                );
                portrait(ui, id, r);
                fit(
                    ui,
                    &format!("J{} · {}", i + 1, FACE_NAMES[id]),
                    Rect::new(r.x, r.bottom(), r.w, 28.),
                    14.,
                    ui.theme.text,
                    true,
                );
            }
            if primary(
                ui,
                "Revanche",
                Rect::new(b.x, b.bottom() - 48., b.w, 48.),
                true,
            ) {
                self.state.faces.next_round();
                self.transition();
            }
            return None;
        }
        let player = match phase {
            FacePhase::Choose { player } | FacePhase::Turn { player } => player,
            _ => unreachable!(),
        };
        self.status(
            ui,
            b,
            &format!("Jogador {}", player + 1),
            self.state.faces.scores(),
        );
        if let Some(id) = self.state.faces.own() {
            portrait(ui, id, Rect::new(b.right() - 48., b.y + 40., 38., 40.));
            fit(
                ui,
                &format!("Meu: {}", FACE_NAMES[id]),
                Rect::new(b.x, b.y + 42., b.w - 52., 32.),
                13.,
                ui.theme.muted,
                false,
            );
        } else {
            fit(
                ui,
                "Escolha seu personagem",
                Rect::new(b.x, b.y + 42., b.w, 32.),
                16.,
                ui.theme.text,
                true,
            );
        }
        let vp = Rect::new(b.x, b.y + 86., b.w, (b.h - 150.).max(48.));
        let cols = if b.w >= 760. {
            6
        } else if b.w >= 480. {
            4
        } else {
            3
        };
        let gap = 8.;
        let cw = (vp.w - gap * (cols - 1) as f32) / cols as f32;
        let ch = (cw * 1.12).clamp(94., 152.);
        let height = FACE_COUNT.div_ceil(cols) as f32 * (ch + gap) - gap;
        let (o, tap) = page_input(&mut self.pan, &mut self.blocked, press, vp, height);
        let old = ui.override_pointer(tap);
        let marks = self.state.faces.marks();
        clip(Some(vp));
        for i in 0..FACE_COUNT {
            let r = Rect::new(
                vp.x + (i % cols) as f32 * (cw + gap),
                o.y + (i / cols) as f32 * (ch + gap),
                cw,
                ch,
            );
            if r.bottom() < vp.y || r.y > vp.bottom() {
                continue;
            }
            let selected =
                matches!(phase, FacePhase::Choose { .. }) && self.state.faces.own() == Some(i);
            bordered(
                r,
                14.,
                if selected {
                    ui.theme.accent
                } else {
                    ui.theme.line
                },
                ui.theme.bg,
            );
            portrait(ui, i, Rect::new(r.x + 4., r.y + 3., r.w - 8., r.h - 31.));
            fit(
                ui,
                FACE_NAMES[i],
                Rect::new(r.x + 4., r.bottom() - 28., r.w - 8., 24.),
                13.,
                ui.theme.text,
                true,
            );
            if marks[i] {
                rounded(
                    Rect::new(r.x + 1., r.y + 1., r.w - 2., r.h - 2.),
                    13.,
                    Color::new(ui.theme.bg.r, ui.theme.bg.g, ui.theme.bg.b, 0.82),
                );
                draw_line(
                    r.x + 18.,
                    r.y + 18.,
                    r.right() - 18.,
                    r.bottom() - 32.,
                    2.,
                    ui.theme.muted,
                );
            }
            if ui.hit(r) {
                match phase {
                    FacePhase::Choose { .. } => {
                        self.state.faces.choose(i);
                        self.save();
                    }
                    FacePhase::Turn { .. } => {
                        if self.guessing {
                            self.guess = Some(i);
                            self.revision += 1;
                        } else {
                            self.state.faces.toggle(i);
                            self.save();
                        }
                    }
                    _ => {}
                }
            }
        }
        clip(None);
        ui.override_pointer(old);
        let footer = Rect::new(b.x, b.bottom() - 48., b.w, 48.);
        if matches!(phase, FacePhase::Choose { .. }) {
            if primary(
                ui,
                "Esconder e passar",
                footer,
                self.state.faces.own().is_some(),
            ) {
                self.state.faces.seal();
                self.transition();
            }
        } else {
            let half = (b.w - 8.) / 2.;
            if ui.button(
                if self.guessing {
                    "Cancelar palpite"
                } else {
                    "Palpite"
                },
                Rect::new(footer.x, footer.y, half, 48.),
                false,
            ) {
                self.guessing = !self.guessing;
                self.revision += 1;
            }
            if primary(
                ui,
                "Passar",
                Rect::new(footer.x + half + 8., footer.y, half, 48.),
                true,
            ) {
                self.state.faces.pass();
                self.transition();
            }
        }
        None
    }
    fn modal(&mut self, ui: &mut Ui, b: Rect) -> Option<Pulse> {
        let w = b.w.min(420.);
        let r = Rect::new(b.center().x - w / 2., b.center().y - 90., w, 180.);
        bordered(r, 20., ui.theme.line, ui.theme.bg);
        let title = self.guess.map_or_else(
            || "Reiniciar partida?".into(),
            |id| format!("É {}?", FACE_NAMES[id]),
        );
        fit(
            ui,
            &title,
            Rect::new(r.x + 12., r.y + 14., r.w - 24., 32.),
            22.,
            ui.theme.text,
            true,
        );
        fit(
            ui,
            if self.reset {
                "A partida e o placar serão apagados."
            } else {
                "Um palpite errado perde a rodada."
            },
            Rect::new(r.x + 12., r.y + 58., r.w - 24., 26.),
            13.,
            ui.theme.muted,
            false,
        );
        let half = (r.w - 36.) / 2.;
        if ui.button(
            "Cancelar",
            Rect::new(r.x + 12., r.bottom() - 60., half, 48.),
            false,
        ) {
            self.reset = false;
            self.guess = None;
            self.revision += 1;
        }
        if primary(
            ui,
            if self.reset { "Reiniciar" } else { "Confirmar" },
            Rect::new(r.center().x + 6., r.bottom() - 60., half, 48.),
            true,
        ) {
            if self.reset {
                self.state.reset(self.kind);
                self.reset = false;
                self.transition();
            } else if let Some(id) = self.guess.take() {
                self.state.faces.guess(id);
                self.transition();
                return Some(Pulse::Won);
            }
        }
        None
    }
    fn rules(&self) -> Vec<&'static str> {
        match self.kind {
            Kind::TicTacToe => vec![
                "Duas pessoas no mesmo aparelho. Jogador 1 usa X; jogador 2 usa O. Alternem os toques em casas vazias.",
                "Faça três símbolos em linha, coluna ou diagonal. Se o tabuleiro encher sem uma linha, a rodada empata.",
                "Teclas 1–9 escolhem casas, da esquerda para a direita, de cima para baixo. Tab e Enter também navegam. A revanche alterna quem começa; ↻ reinicia com confirmação.",
            ],
            Kind::ConnectFour => vec![
                "Duas pessoas no mesmo aparelho. Jogador 1 usa coral; jogador 2 usa azul. Toque numa coluna: a peça cai na casa livre mais baixa.",
                "Faça quatro peças na horizontal, vertical ou diagonal. Colunas cheias não aceitam jogadas. O tabuleiro cheio sem vencedor empata.",
                "Teclas 1–7 escolhem colunas. Tab e Enter também navegam. A revanche alterna quem começa; ↻ reinicia com confirmação.",
            ],
            Kind::Mastermind => vec![
                "Um jogador cria em segredo um código de quatro posições usando seis cores. Cores podem se repetir. Esconda e passe o aparelho para o outro jogador.",
                "O outro jogador tem dez tentativas para descobrir as cores e posições. Cada ponto preenchido indica uma cor na posição correta; cada ponto vazio indica uma cor correta em outra posição. As pistas não correspondem a posições específicas e não contam uma peça duas vezes.",
                "Toque em uma posição e numa cor. As cores têm números para não depender apenas da visão de cores. Teclas 1–6 escolhem cores, setas mudam a posição, Delete apaga e Enter confirma.",
                "Depois da rodada, troquem os papéis. Quem cria o código ganha um ponto por tentativa usada pelo adversário. Compare o placar depois de um número par de rodadas: o maior vence.",
                "Passar, voltar, abrir menus, mudar o tamanho da tela ou interromper o app esconde as telas privadas. O progresso é salvo sem nenhuma conexão de sala.",
            ],
            Kind::Faces => vec![
                "Cada jogador escolhe um dos 24 personagens originais em segredo e esconde a tela antes de passar. Vocês podem escolher o mesmo personagem.",
                "Na sua vez, pergunte em voz alta algo que a outra pessoa possa responder com sim ou não. Use o desenho: óculos, chapéu, cabelo, barba, acessórios ou cores. A outra pessoa responde sobre seu próprio personagem.",
                "Toque nos personagens incompatíveis para eliminá-los do seu tabuleiro. Toque de novo para restaurar. Deslizar só rola a lista. Cada jogador guarda suas próprias eliminações.",
                "Toque em Passar depois da pergunta. Para tentar descobrir a identidade, use Palpite, escolha um personagem e confirme. Acertar ganha a rodada; errar dá a vitória ao adversário.",
                "Meu personagem mostra somente a sua identidade, nunca a do outro jogador. Voltar, menus, interrupções, rotação e recarregamento cobrem as telas privadas. Revanche mantém o placar.",
            ],
        }
    }
    fn draw_rules(&mut self, ui: &mut Ui, press: Option<Vec2>, b: Rect) {
        let rules = self.rules();
        let vp = Rect::new(b.x, b.y, b.w, b.h - 64.);
        let heights: Vec<_> = rules
            .iter()
            .map(|s| {
                crate::online_view::wrapped_lines(ui, s, b.w - 24., 15.).len() as f32 * 24. + 24.
            })
            .collect();
        let (o, _) = page_input(
            &mut self.pan,
            &mut self.blocked,
            press,
            vp,
            heights.iter().sum(),
        );
        clip(Some(vp));
        let mut y = o.y;
        for (s, h) in rules.iter().zip(heights) {
            wrap(
                ui,
                s,
                Rect::new(b.x + 12., y, b.w - 24., h),
                15.,
                ui.theme.text,
                false,
            );
            y += h;
        }
        clip(None);
        if ui.button("Fechar", Rect::new(b.x, b.bottom() - 48., b.w, 48.), false) {
            self.help = false;
            self.pan = BoardPan::default();
            self.revision += 1;
        }
    }
}
struct GridLayout {
    board: Rect,
    info: Rect,
    footer: Rect,
}
fn grid_layout(kind: Kind, b: Rect) -> GridLayout {
    let landscape = b.w >= 480. && b.h < 450.;
    let side = if landscape { 188. } else { 0. };
    let info = if landscape {
        Rect::new(b.right() - side, b.y, side, b.h)
    } else {
        b
    };
    let avail_w = b.w - if landscape { side + 16. } else { 0. };
    let avail_h = b.h - if landscape { 8. } else { 116. };
    let ratio = if kind == Kind::ConnectFour {
        7. / 6.
    } else {
        1.
    };
    let w = avail_w.min(avail_h * ratio).max(64.);
    let h = w / ratio;
    let area = Rect::new(
        b.x,
        b.y + if landscape { 0. } else { 52. },
        avail_w,
        avail_h,
    );
    GridLayout {
        board: Rect::new(area.center().x - w / 2., area.center().y - h / 2., w, h),
        info,
        footer: Rect::new(info.x, b.bottom() - 48., info.w, 48.),
    }
}

fn symbol(c: Vec2, r: f32, value: u8, ink: Color) {
    if value == 1 {
        draw_line(c.x - r, c.y - r, c.x + r, c.y + r, 4., ink);
        draw_line(c.x + r, c.y - r, c.x - r, c.y + r, 4., ink);
    } else if value == 2 {
        draw_circle_lines(c.x, c.y, r, 4., ink);
    }
}
fn peg(ui: &Ui, c: Vec2, r: f32, v: Option<u8>, selected: bool) {
    let ink = v.map_or(ui.theme.line, |i| PEGS[i as usize]);
    if let Some(value) = v {
        draw_circle(c.x, c.y, r, ink);
        ui.centered(
            &(value + 1).to_string(),
            Rect::new(c.x - r, c.y - r, r * 2., r * 2.),
            (r * 0.72).clamp(9., 15.),
            WHITE,
            true,
        );
    } else {
        draw_circle_lines(c.x, c.y, r, 1.5, ui.theme.line);
    }
    if selected {
        draw_circle_lines(c.x, c.y, r + 4., 2., ui.theme.accent);
    }
}
fn draw_feedback(ui: &Ui, c: Vec2, f: Feedback) {
    for i in 0..4 {
        let p = c + vec2((i % 2) as f32 * 13. - 6.5, (i / 2) as f32 * 13. - 6.5);
        if i < f.exact {
            draw_circle(p.x, p.y, 3.5, ui.theme.text);
        } else if i < f.exact + f.colour {
            draw_circle_lines(p.x, p.y, 3.5, 1.5, ui.theme.text);
        } else {
            draw_circle(p.x, p.y, 1.3, ui.theme.line);
        }
    }
}
/// Original flat vector portraits, with independently varied visible attributes.
fn portrait(ui: &Ui, id: usize, r: Rect) {
    let s = (r.w.min(r.h / 1.2) / 100.).max(0.1);
    let c = vec2(r.center().x, r.y + r.h * 0.45);
    let skin = [
        color_u8!(247, 204, 165, 255),
        color_u8!(183, 119, 84, 255),
        color_u8!(119, 76, 56, 255),
        color_u8!(239, 181, 140, 255),
    ][id % 4];
    let hair = [
        color_u8!(47, 37, 40, 255),
        color_u8!(120, 71, 48, 255),
        color_u8!(218, 149, 60, 255),
        color_u8!(158, 71, 45, 255),
        color_u8!(187, 182, 175, 255),
        color_u8!(82, 53, 112, 255),
    ][id % 6];
    let shirt = [BLUE, VIOLET, color_u8!(54, 151, 125, 255), CORAL][(id / 3) % 4];
    let p = |x: f32, y: f32| c + vec2(x * s, y * s);
    let ink = if ui.theme.saver {
        ui.theme.text
    } else {
        color_u8!(57, 43, 43, 255)
    };
    let bust = p(0., 44.);
    draw_ellipse(bust.x, bust.y, 35. * s, 19. * s, 0., shirt);
    let neck = p(0., 27.);
    draw_rectangle(neck.x - 8. * s, neck.y, 16. * s, 16. * s, skin);
    if id / 6 % 2 == 1 {
        let h = p(0., 6.);
        draw_ellipse(h.x, h.y, 31. * s, 37. * s, 0., hair);
    }
    for x in [-26., 26.] {
        let ear = p(x, 4.);
        draw_circle(ear.x, ear.y, 6. * s, skin);
    }
    let head = p(0., 0.);
    draw_ellipse(head.x, head.y, 26. * s, 32. * s, 0., skin);
    let top = p(0., -23.);
    draw_ellipse(top.x, top.y, 27. * s, 12. * s, 0., hair);
    for x in [-18., -7., 5., 17.] {
        let fringe = p(
            x,
            -19. + if id.is_multiple_of(2) {
                x.abs() * 0.18
            } else {
                0.
            },
        );
        draw_circle(fringe.x, fringe.y, 8. * s, hair);
    }
    for x in [-11., 11.] {
        let eye = p(x, 2.);
        draw_circle(eye.x, eye.y, 3.4 * s, ink);
        draw_circle(eye.x + s, eye.y - s, s, WHITE);
        let a = p(x - 4., -7.);
        let z = p(x + 4., -8.);
        draw_line(a.x, a.y, z.x, z.y, 1.8 * s, hair);
    }
    let nose = p(0., 11.);
    draw_ellipse(
        nose.x,
        nose.y,
        3. * s,
        2. * s,
        0.,
        Color::new(skin.r * 0.88, skin.g * 0.85, skin.b * 0.85, 1.),
    );
    let mouth = p(0., 19.);
    draw_ellipse_lines(mouth.x, mouth.y, 6. * s, 3. * s, 0., 1.2 * s, ink);
    if id % 4 < 2 {
        for x in [-12., 12.] {
            let g = p(x, 2.);
            draw_ellipse_lines(g.x, g.y, 10. * s, 8. * s, 0., 2. * s, ink);
        }
        let a = p(-2., 2.);
        let z = p(2., 2.);
        draw_line(a.x, a.y, z.x, z.y, 2. * s, ink);
    }
    if id % 6 == 3 || id % 6 == 5 {
        let m = p(0., 16.);
        draw_ellipse(m.x - 4. * s, m.y, 6. * s, 3. * s, -12., hair);
        draw_ellipse(m.x + 4. * s, m.y, 6. * s, 3. * s, 12., hair);
        if id % 6 == 5 {
            let beard = p(0., 26.);
            draw_ellipse(beard.x, beard.y, 13. * s, 6. * s, 0., hair);
        }
    }
    if id.is_multiple_of(5) {
        let brim = p(0., -28.);
        rounded(
            Rect::new(brim.x - 34. * s, brim.y - 4. * s, 68. * s, 8. * s),
            4. * s,
            shirt,
        );
        rounded(
            Rect::new(brim.x - 21. * s, brim.y - 23. * s, 42. * s, 23. * s),
            7. * s,
            shirt,
        );
        let stripe = p(0., -32.);
        draw_rectangle(stripe.x - 21. * s, stripe.y, 42. * s, 4. * s, WHITE);
    }
    if id % 3 == 1 {
        for x in [-28., 28.] {
            let e = p(x, 13.);
            draw_circle_lines(e.x, e.y, 3.5 * s, 1.3 * s, color_u8!(188, 132, 37, 255));
        }
    }
    if id >= 12 {
        let a = p(-10., 34.);
        let z = p(10., 34.);
        let mid = p(0., 36.);
        draw_triangle(a, p(-10., 42.), mid, ink);
        draw_triangle(z, p(10., 42.), mid, ink);
        draw_circle(mid.x, mid.y, 2.5 * s, WHITE);
    }
    if id % 7 == 2 {
        let e = p(20., -23.);
        draw_circle(e.x, e.y, 6. * s, CORAL);
        draw_circle(e.x, e.y, 2. * s, WHITE);
    }
}
pub fn preview(ui: &Ui, kind: Kind, r: Rect) {
    match kind {
        Kind::Faces => {
            for i in 0..4 {
                let size = r.w * 0.43;
                let cell = Rect::new(
                    r.x + (i % 2) as f32 * r.w * 0.52,
                    r.y + (i / 2) as f32 * r.h * 0.5,
                    size,
                    r.h * 0.45,
                );
                portrait(ui, [0, 7, 13, 20][i], cell);
            }
        }
        Kind::Mastermind => {
            for i in 0..3 {
                let y = r.y + r.h * (0.18 + i as f32 * 0.29);
                for j in 0..4 {
                    peg(
                        ui,
                        vec2(r.x + r.w * (0.12 + j as f32 * 0.19), y),
                        r.w * 0.065,
                        Some([[0, 1, 2, 3], [0, 2, 1, 4], [0, 2, 1, 3]][i][j]),
                        false,
                    );
                }
                draw_feedback(
                    ui,
                    vec2(r.right() - r.w * 0.08, y),
                    [
                        Feedback {
                            exact: 1,
                            colour: 2,
                        },
                        Feedback {
                            exact: 3,
                            colour: 0,
                        },
                        Feedback {
                            exact: 4,
                            colour: 0,
                        },
                    ][i],
                );
            }
        }
        Kind::TicTacToe => {
            let cell = r.w / 3.;
            for i in 0..9 {
                let c = vec2(
                    r.x + (i % 3) as f32 * cell + cell / 2.,
                    r.y + (i / 3) as f32 * cell + cell / 2.,
                );
                symbol(
                    c,
                    cell * 0.23,
                    [1, 2, 0, 0, 1, 2, 0, 0, 1][i],
                    if i % 2 == 0 { CORAL } else { BLUE },
                );
            }
            for n in 1..3 {
                draw_line(
                    r.x + n as f32 * cell,
                    r.y,
                    r.x + n as f32 * cell,
                    r.bottom(),
                    1.,
                    ui.theme.line,
                );
                draw_line(
                    r.x,
                    r.y + n as f32 * cell,
                    r.right(),
                    r.y + n as f32 * cell,
                    1.,
                    ui.theme.line,
                );
            }
        }
        Kind::ConnectFour => {
            let cell = r.w / 7.;
            let y = r.center().y - cell * 3.;
            for row in 0..6 {
                for col in 0..7 {
                    let c = vec2(
                        r.x + (col as f32 + 0.5) * cell,
                        y + (row as f32 + 0.5) * cell,
                    );
                    let value = if row >= 4 && col < 5 {
                        if (row + col) % 2 == 0 { CORAL } else { BLUE }
                    } else {
                        ui.theme.line
                    };
                    draw_circle(c.x, c.y, cell * 0.33, value);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boards_and_footer_fit_phone_tablet_desktop_and_landscape() {
        for (w, h) in [
            (280., 360.),
            (320., 480.),
            (390., 844.),
            (568., 320.),
            (820., 1180.),
            (1440., 900.),
        ] {
            let bw = (w - 32.0_f32).min(900.);
            let b = Rect::new((w - bw) / 2., 66., bw, h - 82.);
            for kind in [Kind::TicTacToe, Kind::ConnectFour] {
                let l = grid_layout(kind, b);
                assert!(
                    l.board.x >= b.x
                        && l.board.y >= b.y
                        && l.board.right() <= b.right() + 0.1
                        && l.board.bottom() <= b.bottom() + 0.1
                );
                assert!(l.footer.y >= b.y && l.footer.bottom() <= b.bottom());
                let (w, _) = if kind == Kind::TicTacToe {
                    (3, 3)
                } else {
                    (7, 6)
                };
                assert!(l.board.w / w as f32 >= 24.);
            }
        }
    }
}
