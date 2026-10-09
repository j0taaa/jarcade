use crate::fih_art::{BLUE, GOLD, PINK};
use crate::{
    fih_art::*,
    platform,
    ui::{Ui, rounded},
};
use jarcade::{
    feedback::Pulse,
    fih::{Care, FOODS, Fih, Outcome, POTIONS, Room, Style},
    fih_ball::Ball,
    fih_games::{Kind, Round},
    fih_interaction::{Action as PetAction, Animation, Lather, Pose, can_feed, mouth_hit},
    layout::{FihLayout, touch_point},
};
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    None,
    Arcade,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Rooms,
    Pantry,
    Shop,
    Wardrobe,
    Games,
    Medicine,
    PotionShop,
}
#[derive(Clone, Copy)]
enum DragTool {
    Food(usize),
    Soap,
    Potion(usize),
}
struct Drag {
    tool: DragTool,
    keyboard: bool,
    started: f64,
    start: Vec2,
    point: Vec2,
}
pub struct FihPage {
    pub pet: Fih,
    pub round: Option<Round>,
    pub revision: u64,
    art: Art,
    room: Room,
    overlay: Overlay,
    nav: Nav,
    style: Style,
    candidate: u8,
    style_page: u8,
    category: usize,
    selected_food: usize,
    selected_potion: usize,
    food_page: usize,
    game_page: usize,
    finger: Option<Vec2>,
    ball: Ball,
    ball_frame: Option<f64>,
    drag: Option<Drag>,
    soap: f32,
    lather: Lather,
    animation: Option<Animation>,
    eaten_food: usize,
    shower_until: f64,
    reaction: f64,
    toast_until: f64,
    message: String,
    last_frame: Option<f64>,
    save_failed: bool,
    refresh: bool,
}
impl FihPage {
    pub fn new() -> Self {
        let pet = platform::load_fih();
        let room = if pet.sleeping {
            Room::Bedroom
        } else {
            Room::Kitchen
        };
        Self {
            pet,
            round: None,
            revision: 0,
            art: Art::shared(),
            room,
            overlay: Overlay::None,
            nav: Nav::None,
            style: Style::Clothes,
            candidate: 0,
            style_page: 0,
            category: 0,
            selected_food: 0,
            selected_potion: 0,
            food_page: 0,
            game_page: 0,
            finger: None,
            ball: Ball::default(),
            ball_frame: None,
            drag: None,
            soap: 0.,
            lather: Lather::default(),
            animation: None,
            eaten_food: 0,
            shower_until: 0.,
            reaction: 0.,
            toast_until: 0.,
            message: String::new(),
            last_frame: None,
            save_failed: false,
            refresh: false,
        }
    }
    fn save(&mut self) {
        self.save_failed = !platform::save_fih(&self.pet);
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn enter(&mut self, now: f64) {
        self.pet.advance(miniquad::date::now());
        self.round = None;
        self.overlay = Overlay::None;
        self.room = if self.pet.sleeping {
            Room::Bedroom
        } else {
            Room::Kitchen
        };
        self.reaction = now + 1.;
        self.drag = None;
        self.nav = Nav::None;
        self.save();
    }
    pub fn interrupt(&mut self) {
        if let Some(r) = &mut self.round {
            r.paused = true;
        }
        self.last_frame = None;
        self.drag = None;
        self.reaction = 0.;
        self.animation = None;
        self.lather.end_stroke();
        self.ball.cancel();
        self.ball_frame = None;
        self.finger = None;
    }
    pub fn back(&mut self) -> bool {
        if self.drag.take().is_some() || self.ball.held {
            self.ball.cancel();
            self.lather.end_stroke();
            self.revision += 1;
            false
        } else if self.round.take().is_some() {
            self.overlay = Overlay::Games;
            self.revision += 1;
            false
        } else if self.overlay != Overlay::None {
            self.overlay = Overlay::None;
            self.revision += 1;
            false
        } else {
            true
        }
    }
    pub fn take_nav(&mut self) -> Nav {
        std::mem::replace(&mut self.nav, Nav::None)
    }
    pub fn needs_frame(&self, now: f64, saver: bool) -> bool {
        self.round
            .as_ref()
            .is_some_and(|r| !r.finished && !r.paused)
            || self.drag.is_some()
            || (self.overlay == Overlay::None
                && self.round.is_none()
                && ((!saver && self.finger.is_some())
                    || (self.room == Room::Playroom && !self.pet.sleeping && self.ball.moving())))
            || (self.round.is_none() && self.overlay == Overlay::Wardrobe && now < self.reaction)
            || (self.round.is_none()
                && self.overlay == Overlay::None
                && (!saver || now < self.reaction.max(self.shower_until)))
    }
    pub fn delay(&mut self, now: f64, saver: bool) -> Option<f64> {
        if std::mem::take(&mut self.refresh) {
            return Some(0.);
        }
        if self.needs_frame(now, saver) {
            Some(if saver { 1. / 30. } else { 0. })
        } else if self.round.is_none() {
            if self.overlay != Overlay::None {
                return (now < self.toast_until).then_some((self.toast_until - now).max(0.));
            }
            let minute = (60. - (miniquad::date::now() - self.pet.updated)).clamp(1., 60.);
            Some(if now < self.toast_until {
                (self.toast_until - now).min(minute)
            } else {
                minute
            })
        } else {
            None
        }
    }
    fn respond(&mut self, result: Outcome, text: &str, now: f64) -> Option<Pulse> {
        self.message = match result {
            Outcome::Changed => text,
            Outcome::Full => "NAH",
            Outcome::Sleeping => "Sleeping",
            Outcome::Poor => "Not enough coins",
            Outcome::Cooldown => "Loved",
            Outcome::Invalid => "Out of stock",
        }
        .into();
        self.toast_until = 0.;
        if result == Outcome::Full && text == "Yum!" {
            self.animation = Some(Animation {
                action: PetAction::Refuse,
                started: now,
            });
            self.reaction = now + 1.6;
            self.revision += 1;
        }
        if result == Outcome::Changed {
            let action = match text {
                "Yum!" => PetAction::Feed,
                "Squeaky clean!" => PetAction::Wash,
                "Feeling better" => PetAction::Heal,
                "Looking lovely!" => PetAction::Dress,
                "Sweet dreams" => PetAction::Sleep,
                "Good morning!" => PetAction::Wake,
                _ => PetAction::Love,
            };
            let animation = Animation {
                action,
                started: now,
            };
            self.reaction = now + animation.duration();
            self.animation = Some(animation);
            self.save();
            Some(Pulse::Eat)
        } else {
            Some(Pulse::Tap)
        }
    }
    pub fn room(&self) -> Room {
        self.room
    }
    pub fn move_room(&mut self, room: Room, ui: &mut Ui) {
        self.ball.cancel();
        self.ball_frame = None;
        self.room = room;
        self.overlay = Overlay::None;
        self.drag = None;
        self.lather.end_stroke();
        self.message.clear();
        self.revision += 1;
        ui.reset_focus();
    }
    fn open(&mut self, overlay: Overlay, ui: &mut Ui) {
        self.overlay = overlay;
        self.drag = None;
        self.revision += 1;
        ui.reset_focus();
        if overlay == Overlay::Wardrobe {
            self.candidate = self.pet.selected(self.style);
            self.style_page = self.candidate / if screen_height() < 550. { 3 } else { 6 };
        }
    }
    fn pet_position(l: &FihLayout) -> (Vec2, f32) {
        let r = (l.pet.w * 0.29).min(l.pet.h * 0.31).clamp(20., 150.);
        (vec2(l.pet.center().x, l.pet.center().y - r * 0.05), r)
    }
    fn hud(&mut self, ui: &mut Ui, l: &FihLayout, enabled: bool) {
        let left = Rect::new(12., 12., 44., 44.);
        if enabled {
            if icon_button(ui, Glyph::Back, left, false) {
                self.nav = Nav::Arcade;
            }
        } else {
            glass(ui, left, 16.);
            glyph(Glyph::Back, left.center(), 14.);
        }
        let coins = self.pet.coins.to_string();
        let level = format!("Lv {}", self.pet.level());
        let total = 16. + ui.text_width(&coins, 11., true) + 18. + ui.text_width(&level, 11., true);
        let x = (screen_width() - total) / 2.;
        draw_circle(x + 6., 57., 6., GOLD);
        ui.heading(
            &coins,
            x + 16.,
            61.,
            11.,
            if ui.theme.saver { WHITE } else { INK },
        );
        ui.heading(
            &level,
            x + 16. + ui.text_width(&coins, 11., true) + 18.,
            61.,
            11.,
            if ui.theme.saver { WHITE } else { INK },
        );
        let stats = [
            (self.pet.food, Glyph::Food, PINK, Room::Kitchen),
            (self.pet.clean, Glyph::Soap, BLUE, Room::Bathroom),
            (self.pet.joy, Glyph::Heart, LILAC, Room::Playroom),
            (self.pet.energy, Glyph::Moon, GOLD, Room::Bedroom),
            (self.pet.health, Glyph::Potion, MINT, Room::Clinic),
        ];
        let cell = l.stats.w / 5.;
        for (i, (value, kind, color, room)) in stats.into_iter().enumerate() {
            let box_rect = Rect::new(l.stats.x + i as f32 * cell, l.stats.y, cell, l.stats.h);
            let c = box_rect.center();
            draw_circle(
                c.x,
                c.y,
                16.,
                if ui.theme.saver {
                    BLACK
                } else {
                    color_u8!(255, 255, 255, 230)
                },
            );
            draw_circle_lines(
                c.x,
                c.y,
                16.,
                2.,
                if ui.theme.saver {
                    ui.theme.line
                } else {
                    color_u8!(227, 220, 214, 255)
                },
            );
            draw_arc(c.x, c.y, 48, 16., -90., 2.8, 360. * value / 100., color);
            glyph(kind, c, 10.);
            if enabled && ui.hit(box_rect) {
                self.move_room(room, ui);
            }
        }
    }
    pub fn draw(&mut self, ui: &mut Ui, press: Option<Vec2>, now: f64) -> Option<Pulse> {
        if miniquad::date::now() - self.pet.updated >= 60. {
            self.pet.advance(miniquad::date::now());
            self.save();
        }
        if self.round.is_some() {
            return self.draw_round(ui, press, now);
        }
        let l = FihLayout::new(screen_width(), screen_height());
        self.art.room(
            l.viewport,
            self.room,
            self.pet.background,
            ui.theme.saver,
            self.pet.sleeping && self.room == Room::Bedroom,
        );
        let (center, radius) = Self::pet_position(&l);
        self.finger = active_pointer().filter(|p| {
            self.overlay == Overlay::None && p.y > l.room_nav.bottom() && !l.tools.contains(*p)
        });
        self.ball.resize(l.pet.h / l.pet.w);
        let ball_dt = self.ball_frame.replace(now).map_or(0., |last| now - last);
        if self.room == Room::Playroom && self.overlay == Overlay::None && !self.pet.sleeping {
            self.ball.advance(ball_dt);
        }

        if !ui.theme.saver {
            ellipse(
                center + vec2(0., radius * 1.62),
                vec2(radius * 0.62, radius * 0.08),
                Color::new(0.25, 0.18, 0.20, 0.1),
            );
        }
        let mut pose = if ui.theme.saver || self.overlay != Overlay::None {
            Pose::default()
        } else {
            Pose::idle(now)
        };
        pose.needs(&self.pet);
        if let Some(p) = self.finger {
            let q = (p - center) / radius;
            pose.gaze((q.x, q.y));
        }
        if self.room == Room::Playroom {
            let p = l.pet.point() + vec2(self.ball.x, self.ball.y) * l.pet.w;
            let q = (p - center) / radius;
            pose.gaze((q.x, q.y));
            if self.ball.moving() && !self.pet.sleeping {
                pose.delight = 0.5;
                pose.sad = 0.;
            }
        }
        if let Some(animation) = self.animation {
            animation.apply(&mut pose, now);
        }
        if let Some(drag) = &self.drag {
            match drag.tool {
                DragTool::Food(_) => {
                    let q = (drag.point - center) / radius;
                    pose.watch((q.x, q.y));
                }
                DragTool::Potion(_) => {
                    let q = (drag.point - center) / radius;
                    pose.watch((q.x, q.y));
                }
                DragTool::Soap => {
                    pose.blink = 0.35;
                    pose.look = (0., 0.5);
                    pose.delight = self.soap * 0.15;
                }
            }
        }
        self.art.fish_pose(center, radius, &self.pet, pose);
        if let Some(animation) = self.animation.filter(|a| a.active(now)) {
            let t = animation.progress(now);
            if animation.action == PetAction::Feed && t < 0.28 {
                food_icon(
                    self.eaten_food,
                    center + vec2(0., radius * 0.28),
                    radius * 0.17 * (1. - t / 0.28),
                );
            }
            if matches!(
                animation.action,
                PetAction::Love
                    | PetAction::Heal
                    | PetAction::Dress
                    | PetAction::Wake
                    | PetAction::Wash
            ) {
                for i in 0..5 {
                    let a = i as f32 * 1.26;
                    let p = center
                        + vec2(
                            a.cos() * radius * (0.8 + t * 0.6),
                            -radius * (0.5 + t * 0.9) + a.sin() * radius * 0.35,
                        );
                    if animation.action == PetAction::Love {
                        heart(p, radius * 0.07 * (1. - t), PINK);
                    } else {
                        draw_poly(
                            p.x,
                            p.y,
                            4,
                            radius * 0.07 * (1. - t),
                            45.,
                            if animation.action == PetAction::Heal {
                                MINT
                            } else {
                                GOLD
                            },
                        );
                    }
                }
            }
        }
        if self
            .animation
            .is_some_and(|a| a.action == PetAction::Refuse && a.active(now))
        {
            let speech = Rect::new(
                (center.x + radius * 0.55).min(screen_width() - 86.),
                (center.y - radius * 0.85).max(115.),
                72.,
                36.,
            );
            glass(ui, speech, 16.);
            ui.centered(
                "NAH",
                speech,
                15.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
        }
        if self.room == Room::Bathroom {
            for (i, &(x, y)) in self.lather.spots.iter().enumerate() {
                bubble(
                    center + vec2(x, y) * radius,
                    radius * (0.045 + (i % 3) as f32 * 0.012),
                    if ui.theme.saver { BLUE } else { WHITE },
                );
            }
        }
        if self.room == Room::Playroom {
            let p = l.pet.point() + vec2(self.ball.x, self.ball.y) * l.pet.w;
            ball_icon(p, self.ball.radius * l.pet.w, (now as f32) * self.ball.vx);
        }
        if self.room == Room::Bathroom && now < self.shower_until {
            for i in 0..12 {
                let x = center.x - radius * 0.7 + i as f32 * radius * 0.12;
                let y =
                    center.y - radius + (((now * 200.) as f32 + i as f32 * 37.) % (radius * 1.7));
                draw_line(x, y, x - 3., y + 10., 2., BLUE);
            }
        }
        if self.pet.sleeping && self.room == Room::Bedroom {
            ui.heading(
                "z",
                center.x + radius * 0.6,
                center.y - radius * 0.6,
                20.,
                WHITE,
            );
            ui.heading(
                "Z",
                center.x + radius * 0.85,
                center.y - radius * 0.9,
                26.,
                WHITE,
            );
        }
        let enabled = self.overlay == Overlay::None;
        let keyboard_care = self.drag.as_ref().is_some_and(|drag| drag.keyboard);
        let focus = ui.keyboard_focus;
        if keyboard_care {
            ui.keyboard_focus = false;
        }
        self.hud(ui, &l, enabled);
        let result = if enabled {
            self.room_controls(ui, &l, press, now, center, radius)
        } else {
            let result = match self.overlay {
                Overlay::Rooms => self.rooms_menu(ui),
                Overlay::Pantry => self.pantry(ui, press, now),
                Overlay::Shop => self.food_shop(ui, now),
                Overlay::Medicine | Overlay::PotionShop => self.medicine(ui, press, now),
                Overlay::Wardrobe => self.wardrobe(ui, now),
                Overlay::Games => self.games_menu(ui, now),
                Overlay::None => None,
            };
            self.draw_toast(ui, &l, now);
            result
        };
        if keyboard_care {
            ui.keyboard_focus = focus;
        }
        result
    }
    fn room_controls(
        &mut self,
        ui: &mut Ui,
        l: &FihLayout,
        press: Option<Vec2>,
        now: f64,
        center: Vec2,
        r: f32,
    ) -> Option<Pulse> {
        let mut pulse = None;
        let nav = l.room_nav;
        if icon_button(ui, Glyph::Back, Rect::new(nav.x, nav.y, 44., 44.), false)
            || (self.drag.is_none()
                && !self.ball.held
                && !ui.keyboard_focus
                && is_key_pressed(KeyCode::Left))
        {
            self.move_room(self.room.neighbor(-1), ui);
        }
        if icon_button(
            ui,
            Glyph::Next,
            Rect::new(nav.right() - 44., nav.y, 44., 44.),
            false,
        ) || (self.drag.is_none()
            && !self.ball.held
            && !ui.keyboard_focus
            && is_key_pressed(KeyCode::Right))
        {
            self.move_room(self.room.neighbor(1), ui);
        }
        let title = Rect::new(nav.x + 52., nav.y, nav.w - 104., 44.);
        glass(ui, title, 20.);
        ui.centered(
            self.room.title(),
            title,
            15.,
            if ui.theme.saver { WHITE } else { INK },
            true,
        );
        if ui.hit(title) {
            self.open(Overlay::Rooms, ui);
            return Some(Pulse::Tap);
        }
        let landscape = screen_height() < 520. && screen_width() > screen_height() * 1.3;
        let tools = l.tools;
        glass(ui, tools, 24.);
        let count = self.room.tools().len();
        let cell = tools.w / count as f32;
        let action_rect = |i: usize| {
            if landscape {
                Rect::new(
                    tools.x + 12.,
                    tools.y + 10. + i as f32 * (tools.h - 18.) / count as f32,
                    48.,
                    48.,
                )
            } else {
                Rect::new(
                    tools.x + cell * (i as f32 + 0.5) - 26.,
                    tools.y + 7.,
                    52.,
                    52.,
                )
            }
        };
        let a = action_rect(0);
        let b = action_rect(1);
        let mut labels: Vec<&str> = Vec::new();
        match self.room {
            Room::Kitchen => {
                labels.extend(["Pantry", FOODS[self.selected_food].name, "Shop"]);
                if icon_button(ui, Glyph::Fridge, a, false) {
                    self.open(Overlay::Pantry, ui);
                }
                glass(ui, b, 17.);
                food_icon(self.selected_food, b.center(), 21.);
                let food_pressed = ui.hit(b);
                if self.drag.is_none() && food_pressed {
                    if self.selected_food != 0 && self.pet.pantry[self.selected_food] == 0 {
                        self.open(Overlay::Shop, ui);
                    } else if self.pet.sleeping {
                        pulse = self.respond(Outcome::Sleeping, "", now);
                    } else {
                        self.start_drag(DragTool::Food(self.selected_food), press, b.center());
                    }
                }
                if icon_button(ui, Glyph::Shop, action_rect(2), false) {
                    self.open(Overlay::Shop, ui);
                }
            }
            Room::Bathroom => {
                labels.extend(["Soap", "Rinse"]);
                let soap_pressed = icon_button(ui, Glyph::Soap, a, self.soap > 0.);
                if self.drag.is_none() && soap_pressed && !self.pet.sleeping {
                    self.start_drag(DragTool::Soap, press, center);
                }
                if icon_button(ui, Glyph::Shower, b, self.lather.ready()) && self.lather.ready() {
                    let result = self.pet.care(Care::Wash, miniquad::date::now());
                    if matches!(result, Outcome::Changed | Outcome::Full) {
                        self.soap = 0.;
                        self.lather.clear();
                        self.shower_until = now + 1.6;
                        self.reaction = now + 1.6;
                        self.animation = Some(Animation {
                            action: PetAction::Wash,
                            started: now,
                        });
                    }
                    pulse = self.respond(result, "Squeaky clean!", now);
                }
            }
            Room::Bedroom => {
                labels.extend([if self.pet.sleeping { "Wake" } else { "Sleep" }, "Wardrobe"]);
                if icon_button(
                    ui,
                    if self.pet.sleeping {
                        Glyph::Sun
                    } else {
                        Glyph::Moon
                    },
                    a,
                    self.pet.sleeping,
                ) {
                    let result = self.pet.care(Care::Sleep, miniquad::date::now());
                    pulse = self.respond(
                        result,
                        if self.pet.sleeping {
                            "Sweet dreams"
                        } else {
                            "Good morning!"
                        },
                        now,
                    );
                }
                if icon_button(ui, Glyph::Shirt, b, false) {
                    self.open(Overlay::Wardrobe, ui);
                }
            }
            Room::Playroom => {
                labels.extend(["Mini-games", "Ball"]);
                if icon_button(ui, Glyph::Play, a, false) && !self.pet.sleeping {
                    self.open(Overlay::Games, ui);
                }
                glass(ui, b, 17.);
                ball_icon(b.center(), 17., 0.);
                if ui.hit(b) && !self.pet.sleeping {
                    self.ball = Ball::default();
                    self.ball.resize(l.pet.h / l.pet.w);
                    self.revision += 1;
                }
                if let Some(p) = press.filter(|p| l.pet.contains(*p)) {
                    let q = (p - l.pet.point()) / l.pet.w;
                    if self.ball.hit(q.x, q.y) && !self.pet.sleeping {
                        self.ball.grab(now);
                    }
                }
                if self.ball.held {
                    if touches().iter().any(|t| t.phase == TouchPhase::Cancelled)
                        || touches().len() > 1
                    {
                        self.ball.cancel();
                    } else if let Some(p) = active_pointer() {
                        let q = (p - l.pet.point()) / l.pet.w;
                        self.ball.drag(q.x, q.y, now);
                    } else if self.ball.release(now) {
                        let result = self.pet.care(Care::Pet, miniquad::date::now());
                        pulse = self.respond(result, "Let's play!", now);
                    }
                }
            }
            Room::Clinic => {
                labels.extend(["Cabinet", "Potion shop"]);
                if icon_button(ui, Glyph::Potion, a, false) {
                    self.open(Overlay::Medicine, ui);
                }
                if icon_button(ui, Glyph::Shop, b, false) {
                    self.open(Overlay::PotionShop, ui);
                }
            }
        }
        for (i, text) in labels.into_iter().enumerate() {
            let item = action_rect(i);
            let label = if landscape {
                Rect::new(
                    item.right() + 5.,
                    item.y + 8.,
                    tools.right() - item.right() - 10.,
                    30.,
                )
            } else {
                Rect::new(tools.x + i as f32 * cell, tools.y + 64., cell, 18.)
            };
            ui.centered(
                text,
                label,
                11.,
                if ui.theme.saver { ui.theme.muted } else { INK },
                false,
            );
        }
        if self.overlay == Overlay::None {
            pulse = self.update_drag(ui, now, center, r).or(pulse);
        }
        self.draw_toast(ui, l, now);
        pulse
    }
    fn start_drag(&mut self, tool: DragTool, press: Option<Vec2>, fallback: Vec2) {
        let point = press.unwrap_or(fallback);
        self.drag = Some(Drag {
            tool,
            keyboard: press.is_none(),
            started: get_time(),
            start: point,
            point,
        });
        self.revision += 1;
    }
    fn update_drag(&mut self, ui: &Ui, now: f64, center: Vec2, r: f32) -> Option<Pulse> {
        let mut drag = self.drag.take()?;
        let ts = touches();
        if ts.iter().any(|t| t.phase == TouchPhase::Cancelled) || ts.len() > 1 {
            self.lather.end_stroke();
            return None;
        }
        let mut released = active_pointer().is_none();
        if drag.keyboard {
            for (key, d) in [
                (KeyCode::Left, vec2(-1., 0.)),
                (KeyCode::Right, vec2(1., 0.)),
                (KeyCode::Up, vec2(0., -1.)),
                (KeyCode::Down, vec2(0., 1.)),
            ] {
                if is_key_pressed(key) {
                    drag.point += d * r * 0.13;
                }
            }
            released = is_key_pressed(KeyCode::Enter) && now - drag.started > 0.05;
        } else if let Some(t) = ts.iter().find(|t| t.phase != TouchPhase::Cancelled) {
            drag.point = touch_point(t.position, screen_dpi_scale());
        } else {
            let (x, y) = mouse_position();
            drag.point = vec2(x, y);
        }
        let q = (drag.point - center) / r;
        match drag.tool {
            DragTool::Food(index) | DragTool::Potion(index) => {
                if released {
                    let start = (drag.start - center) / r;
                    if can_feed((start.x, start.y), (q.x, q.y), false) {
                        let (result, text) = if matches!(drag.tool, DragTool::Food(_)) {
                            self.eaten_food = index;
                            (self.pet.eat(index, miniquad::date::now()), "Yum!")
                        } else {
                            (
                                self.pet.drink_potion(index, miniquad::date::now()),
                                "Feeling better",
                            )
                        };
                        return self.respond(result, text, now);
                    }
                } else {
                    if matches!(drag.tool, DragTool::Food(_)) {
                        food_icon(index, drag.point, 26.);
                    } else {
                        potion_icon(index, drag.point, 24.);
                    }
                    if mouth_hit((q.x, q.y)) {
                        draw_circle_lines(center.x, center.y + r * 0.28, r * 0.22, 1.5, MINT);
                    }
                    self.drag = Some(drag);
                }
            }
            DragTool::Soap => {
                if released {
                    self.lather.end_stroke();
                } else {
                    let was = self.lather.ready();
                    self.lather.rub((q.x, q.y));
                    self.soap = self.lather.amount;
                    glyph(Glyph::Soap, drag.point, 24.);
                    self.drag = Some(drag);
                    if !was && self.lather.ready() {
                        return Some(Pulse::Tap);
                    }
                }
            }
        }
        let _ = ui;
        None
    }
    fn draw_toast(&self, ui: &Ui, l: &FihLayout, _now: f64) {
        if self.save_failed {
            let text = if self.save_failed {
                "Progress could not be saved"
            } else {
                &self.message
            };
            let width = (ui.text_width(text, 12., false) + 28.).min(screen_width() - 20.);
            let rect = Rect::new((screen_width() - width) / 2., l.tools.y - 40., width, 30.);
            glass(ui, rect, 14.);
            ui.centered(
                text,
                rect,
                12.,
                if ui.theme.saver { WHITE } else { INK },
                false,
            );
        }
    }
    fn panel(&mut self, ui: &mut Ui, title: &str) -> Option<Rect> {
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0.05, 0.04, 0.08, 0.38),
        );
        let width = (screen_width() - 24.).min(720.);
        let height = if matches!(self.overlay, Overlay::Pantry | Overlay::Medicine) {
            (screen_height() * 0.50)
                .clamp(320., 480.)
                .min(screen_height() - 24.)
        } else {
            (screen_height() - 24.).min(720.)
        };
        let rect = Rect::new(
            (screen_width() - width) / 2.,
            if matches!(self.overlay, Overlay::Pantry | Overlay::Medicine) {
                screen_height() - height - 12.
            } else {
                (screen_height() - height) / 2.
            },
            width,
            height,
        );
        glass(ui, rect, 28.);
        ui.heading(
            title,
            rect.x + 20.,
            rect.y + 39.,
            20.,
            if ui.theme.saver { WHITE } else { INK },
        );
        if icon_button(
            ui,
            Glyph::Close,
            Rect::new(rect.right() - 56., rect.y + 12., 44., 44.),
            false,
        ) {
            self.overlay = Overlay::None;
            self.revision += 1;
            ui.reset_focus();
            return None;
        }
        Some(rect)
    }
    fn rooms_menu(&mut self, ui: &mut Ui) -> Option<Pulse> {
        let rect = self.panel(ui, "Fih's home")?;
        glyph(Glyph::Home, vec2(rect.x + 170., rect.y + 30.), 15.);
        let cols = if rect.w > rect.h * 1.3 { 3 } else { 2 };
        let rows = 5_usize.div_ceil(cols);
        let cw = (rect.w - 40. - 8. * (cols - 1) as f32) / cols as f32;
        let ch = (rect.h - 88. - 8. * (rows - 1) as f32) / rows as f32;
        for (i, room) in Room::ALL.into_iter().enumerate() {
            let card = Rect::new(
                rect.x + 20. + (i % cols) as f32 * (cw + 8.),
                rect.y + 68. + (i / cols) as f32 * (ch + 8.),
                cw,
                ch,
            );
            let image = Rect::new(card.x + 4., card.y + 4., card.w - 8., card.h - 35.);
            self.art
                .room(image, room, self.pet.background, ui.theme.saver, false);
            ui.centered(
                room.title(),
                Rect::new(card.x, card.bottom() - 28., card.w, 22.),
                13.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
            if ui.hit(card) {
                self.move_room(room, ui);
                return Some(Pulse::Tap);
            }
        }
        None
    }
    fn pantry(&mut self, ui: &mut Ui, press: Option<Vec2>, _now: f64) -> Option<Pulse> {
        let rect = self.panel(ui, "Pantry")?;
        let foods: Vec<_> = (0..FOODS.len())
            .filter(|&i| i == 0 || self.pet.pantry[i] > 0)
            .collect();
        let landscape = rect.w > rect.h * 1.3;
        let cols = 3;
        let size = if landscape { 3 } else { 6 };
        let pages = foods.len().div_ceil(size).max(1);
        self.food_page %= pages;
        let rows = size / cols;
        let cw = (rect.w - 48.) / cols as f32;
        let ch = (rect.h - 144. - 8. * (rows - 1) as f32) / rows as f32;
        for (slot, &index) in foods
            .iter()
            .skip(self.food_page * size)
            .take(size)
            .enumerate()
        {
            let card = Rect::new(
                rect.x + 16. + (slot % cols) as f32 * (cw + 8.),
                rect.y + 66. + (slot / cols) as f32 * (ch + 8.),
                cw,
                ch,
            );
            rounded(
                card,
                16.,
                if ui.theme.saver {
                    BLACK
                } else {
                    color_u8!(253, 244, 225, 255)
                },
            );
            food_icon(
                index,
                vec2(card.center().x, card.y + ch * 0.40),
                ch.min(cw) * 0.26,
            );
            ui.centered(
                FOODS[index].name,
                Rect::new(card.x, card.bottom() - 35., card.w, 19.),
                11.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
            ui.centered(
                &if index == 0 {
                    "∞".into()
                } else {
                    format!("× {}", self.pet.pantry[index])
                },
                Rect::new(card.x, card.bottom() - 18., card.w, 15.),
                10.,
                if ui.theme.saver { WHITE } else { INK },
                false,
            );
            if ui.hit(card) {
                self.selected_food = index;
                let pointer = press;
                self.overlay = Overlay::None;
                ui.reset_focus();
                self.start_drag(DragTool::Food(index), pointer, card.center());
                return Some(Pulse::Tap);
            }
        }
        let y = rect.bottom() - 60.;
        if icon_button(ui, Glyph::Back, Rect::new(rect.x + 16., y, 44., 44.), false) {
            self.food_page = (self.food_page + pages - 1) % pages;
            self.revision += 1;
        }
        if icon_button(
            ui,
            Glyph::Next,
            Rect::new(rect.right() - 60., y, 44., 44.),
            false,
        ) {
            self.food_page = (self.food_page + 1) % pages;
            self.revision += 1;
        }
        let shop = Rect::new(rect.center().x - 64., y, 128., 44.);
        if ui.button("Food shop", shop, false) {
            self.open(Overlay::Shop, ui);
            self.food_page = 0;
        }
        None
    }
    fn food_shop(&mut self, ui: &mut Ui, _now: f64) -> Option<Pulse> {
        let rect = self.panel(ui, "Food shop")?;
        let cats = ["Fresh", "Meals", "Treats", "Drinks"];
        let tw = (rect.w - 32.) / 4.;
        for (i, name) in cats.into_iter().enumerate() {
            let tab = Rect::new(rect.x + 16. + i as f32 * tw, rect.y + 66., tw - 3., 44.);
            rounded(
                tab,
                12.,
                if ui.theme.saver {
                    BLACK
                } else if self.category == i {
                    color_u8!(220, 237, 230, 255)
                } else {
                    color_u8!(245, 247, 243, 255)
                },
            );
            ui.centered(
                name,
                tab,
                11.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
            if ui.hit(tab) {
                self.category = i;
                self.food_page = 0;
                self.revision += 1;
            }
        }
        let foods: Vec<_> = FOODS
            .iter()
            .enumerate()
            .filter(|(i, f)| *i > 0 && f.category == self.category)
            .collect();
        let size = if rect.h < 500. { 2 } else { 4 };
        let pages = foods.len().div_ceil(size).max(1);
        self.food_page %= pages;
        let ch = ((rect.h - 184.) / size as f32).clamp(54., 112.);
        let mut pulse = None;
        for (slot, (index, food)) in foods
            .iter()
            .skip(self.food_page * size)
            .take(size)
            .enumerate()
        {
            let row = Rect::new(
                rect.x + 16.,
                rect.y + 120. + slot as f32 * (ch + 6.),
                rect.w - 32.,
                ch - 4.,
            );
            rounded(
                row,
                14.,
                if ui.theme.saver {
                    BLACK
                } else {
                    color_u8!(242, 248, 244, 255)
                },
            );
            food_icon(*index, vec2(row.x + 25., row.center().y), 21.);
            ui.heading(
                food.name,
                row.x + 54.,
                row.y + 22.,
                12.,
                if ui.theme.saver { WHITE } else { INK },
            );
            ui.label(
                &format!(
                    "+{:.0} food  ·  ×{}",
                    food.nutrition, self.pet.pantry[*index]
                ),
                row.x + 54.,
                row.y + 40.,
                10.,
                if ui.theme.saver { ui.theme.muted } else { INK },
            );
            let buy = Rect::new(row.right() - 86., row.center().y - 22., 78., 44.);
            if ui.button(
                &format!("Buy · {}", food.price),
                buy,
                self.pet.coins >= food.price,
            ) {
                let result = self.pet.buy_food(*index);
                if result == Outcome::Changed {
                    self.save();
                    pulse = Some(Pulse::Tap);
                }
                self.message = if result == Outcome::Changed {
                    "Stock added"
                } else {
                    "Not enough coins"
                }
                .into();
            }
        }
        let y = rect.bottom() - 56.;
        if icon_button(ui, Glyph::Back, Rect::new(rect.x + 16., y, 44., 44.), false) {
            self.food_page = (self.food_page + pages - 1) % pages;
            self.revision += 1;
        }
        if icon_button(ui, Glyph::Next, Rect::new(rect.x + 66., y, 44., 44.), false) {
            self.food_page = (self.food_page + 1) % pages;
            self.revision += 1;
        }
        ui.centered(
            &format!("{} coins", self.pet.coins),
            Rect::new(rect.x + 116., y, rect.w - 192., 44.),
            11.,
            if ui.theme.saver { WHITE } else { INK },
            true,
        );
        if icon_button(
            ui,
            Glyph::Fridge,
            Rect::new(rect.right() - 60., y, 44., 44.),
            false,
        ) {
            self.open(Overlay::Pantry, ui);
            self.food_page = 0;
        }
        pulse
    }
    fn medicine(&mut self, ui: &mut Ui, press: Option<Vec2>, now: f64) -> Option<Pulse> {
        let shop = self.overlay == Overlay::PotionShop;
        let rect = self.panel(ui, if shop { "Potion shop" } else { "Cabinet" })?;
        let mut pulse = None;
        if shop {
            let ch = ((rect.h - 132.) / 3.).clamp(48., 140.);
            for (i, potion) in POTIONS.iter().enumerate() {
                let row = Rect::new(
                    rect.x + 16.,
                    rect.y + 66. + i as f32 * (ch + 5.),
                    rect.w - 32.,
                    ch - 4.,
                );
                rounded(
                    row,
                    16.,
                    if ui.theme.saver {
                        BLACK
                    } else {
                        color_u8!(243, 248, 242, 255)
                    },
                );
                potion_icon(i, vec2(row.x + 25., row.center().y), 20.);
                ui.heading(
                    potion.name,
                    row.x + 52.,
                    row.y + 21.,
                    12.,
                    if ui.theme.saver { WHITE } else { INK },
                );
                let benefit = match i {
                    0 => "+30 health",
                    1 => "+35 energy",
                    _ => "+45 health",
                };
                ui.label(
                    benefit,
                    row.x + 52.,
                    row.y + 39.,
                    9.,
                    if ui.theme.saver { WHITE } else { INK },
                );
                if i == 2 {
                    ui.label(
                        "+25 rest · +15 food",
                        row.x + 52.,
                        row.y + 53.,
                        9.,
                        if ui.theme.saver { WHITE } else { INK },
                    );
                }
                let button = Rect::new(row.right() - 88., row.center().y - 22., 80., 44.);
                if ui.button(
                    &format!("Buy · {}", potion.price),
                    button,
                    self.pet.coins >= potion.price,
                ) {
                    let result = self.pet.buy_potion(i);
                    if result == Outcome::Changed {
                        self.save();
                        pulse = Some(Pulse::Tap);
                    }
                }
            }
            if ui.button(
                &format!("Cabinet · {} coins", self.pet.coins),
                Rect::new(rect.x + 16., rect.bottom() - 56., rect.w - 32., 44.),
                false,
            ) {
                self.open(Overlay::Medicine, ui);
            }
        } else {
            let cw = (rect.w - 48.) / 3.;
            let ch = rect.h - 142.;
            for (i, potion) in POTIONS.iter().enumerate() {
                let card = Rect::new(rect.x + 16. + i as f32 * (cw + 8.), rect.y + 66., cw, ch);
                rounded(
                    card,
                    16.,
                    if ui.theme.saver {
                        BLACK
                    } else {
                        color_u8!(239, 246, 237, 255)
                    },
                );
                potion_icon(
                    i,
                    vec2(card.center().x, card.y + ch * 0.42),
                    ch.min(cw) * 0.28,
                );
                ui.centered(
                    potion.name,
                    Rect::new(card.x, card.bottom() - 42., cw, 21.),
                    11.,
                    if ui.theme.saver { WHITE } else { INK },
                    true,
                );
                ui.centered(
                    &format!("× {}", self.pet.potions[i]),
                    Rect::new(card.x, card.bottom() - 22., cw, 18.),
                    11.,
                    if ui.theme.saver { WHITE } else { INK },
                    false,
                );
                if ui.hit(card) {
                    if self.pet.potions[i] == 0 {
                        self.open(Overlay::PotionShop, ui);
                    } else if self.pet.sleeping {
                        pulse = self.respond(Outcome::Sleeping, "", now);
                    } else {
                        self.selected_potion = i;
                        let pointer = press;
                        self.overlay = Overlay::None;
                        ui.reset_focus();
                        self.start_drag(DragTool::Potion(i), pointer, card.center());
                    }
                }
            }
            if ui.button(
                "Buy potions",
                Rect::new(rect.x + 16., rect.bottom() - 56., rect.w - 32., 44.),
                true,
            ) {
                self.open(Overlay::PotionShop, ui);
            }
        }
        pulse
    }
    fn preview_pet(&self, index: u8) -> Fih {
        let mut pet = self.pet.clone();
        pet.sleeping = false;
        match self.style {
            Style::Color => pet.color = index,
            Style::Clothes => pet.clothes = index,
            Style::Hat => pet.hat = index,
            Style::Background => pet.background = index,
        };
        pet
    }
    fn style_name(&self, index: u8) -> &'static str {
        match self.style {
            Style::Color => [
                "Peach", "Lilac", "Sky", "Mint", "Honey", "Rose", "Pearl", "Slate",
            ][usize::from(index)],
            Style::Clothes => CLOTHES[usize::from(index)],
            Style::Hat => HATS[usize::from(index)],
            Style::Background => WALLS[usize::from(index)],
        }
    }
    fn wardrobe(&mut self, ui: &mut Ui, now: f64) -> Option<Pulse> {
        let rect = self.panel(ui, "Wardrobe")?;
        let compact = rect.h < 550.;
        let landscape = rect.h < 400. && rect.w > rect.h * 1.3;
        let narrow = rect.h < 400. && !landscape;
        let ph = if narrow {
            96.
        } else if landscape {
            rect.h - 142.
        } else if compact {
            104.
        } else {
            (rect.h * 0.26).min(194.)
        };
        let preview = Rect::new(
            rect.x + 16.,
            rect.y + 66.,
            if narrow {
                rect.w * 0.42
            } else if landscape {
                rect.w * 0.30
            } else {
                rect.w - 32.
            },
            ph,
        );
        let content = if landscape {
            Rect::new(
                preview.right() + 12.,
                rect.y + 64.,
                rect.right() - preview.right() - 28.,
                rect.h - 64.,
            )
        } else {
            rect
        };
        let pet = self.preview_pet(self.candidate);
        self.art
            .room(preview, self.room, pet.background, ui.theme.saver, false);
        let mut pose = Pose::default();
        if let Some(animation) = self.animation.filter(|a| a.action == PetAction::Dress) {
            animation.apply(&mut pose, now);
        }
        self.art.fish_pose(
            vec2(preview.center().x, preview.y + (ph - 26.) * 0.48),
            (ph - 30.) / 3.1,
            &pet,
            pose,
        );
        let name = self.style_name(self.candidate);
        let name_rect = Rect::new(preview.x + 6., preview.bottom() - 26., preview.w - 12., 22.);
        glass(ui, name_rect, 10.);
        ui.centered(
            name,
            name_rect,
            11.,
            if ui.theme.saver { WHITE } else { INK },
            true,
        );
        let kinds = [
            (Style::Color, Glyph::Palette),
            (Style::Clothes, Glyph::Shirt),
            (Style::Hat, Glyph::Hat),
            (Style::Background, Glyph::Wall),
        ];
        let tw = if narrow {
            (rect.right() - preview.right() - 34.) / 2.
        } else {
            (content.w - 50.) / 4.
        };
        let top = if narrow {
            preview.y
        } else if landscape {
            content.y
        } else {
            preview.bottom() + 12.
        };
        for (i, (style, kind)) in kinds.into_iter().enumerate() {
            if icon_button(
                ui,
                kind,
                if narrow {
                    Rect::new(
                        preview.right() + 12. + (i % 2) as f32 * (tw + 6.),
                        top + (i / 2) as f32 * 50.,
                        tw,
                        44.,
                    )
                } else {
                    Rect::new(content.x + 16. + i as f32 * (tw + 6.), top, tw, 44.)
                },
                style == self.style,
            ) {
                self.style = style;
                self.candidate = self.pet.selected(style);
                self.style_page = self.candidate / if screen_height() < 550. { 3 } else { 6 };
                self.revision += 1;
            }
        }
        let grid_top = if narrow {
            preview.bottom() + 12.
        } else {
            top + 56.
        };
        let grid_bottom = rect.bottom() - if landscape { 94. } else { 112. };
        let page_size = if compact { 3 } else { 6 };
        let ch = if compact {
            grid_bottom - grid_top
        } else {
            (grid_bottom - grid_top - 8.) / 2.
        };
        let cw = (content.w - 48.) / 3.;
        for slot in 0..page_size {
            let index = self.style_page * page_size + slot;
            if index >= self.style.count() {
                continue;
            }
            let card = Rect::new(
                content.x + 16. + f32::from(slot % 3) * (cw + 8.),
                grid_top + f32::from(slot / 3) * (ch + 8.),
                cw,
                ch,
            );
            glass(ui, card, 16.);
            if self.style == Style::Background {
                self.art.room(
                    Rect::new(card.x + 5., card.y + 5., card.w - 10., card.h - 30.),
                    self.room,
                    index,
                    ui.theme.saver,
                    false,
                );
            } else {
                let pet = self.preview_pet(index);
                let r = (card.w * 0.33).min((card.h - 30.) / 3.1);
                self.art.fish(
                    vec2(card.center().x, card.y + (card.h - 26.) * 0.48),
                    r,
                    &pet,
                    0.,
                );
            }
            ui.centered(
                self.style_name(index),
                Rect::new(card.x + 2., card.bottom() - 26., card.w - 4., 15.),
                if compact { 8. } else { 10. },
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
            if self.pet.owns(self.style, index) {
                draw_circle(card.right() - 10., card.y + 10., 3., MINT);
            } else {
                ui.centered(
                    &format!("{}", self.style.price(index)),
                    Rect::new(card.x, card.bottom() - 12., card.w, 12.),
                    8.,
                    if ui.theme.saver { WHITE } else { INK },
                    false,
                );
            }
            if index == self.candidate {
                draw_rectangle_lines(
                    card.x + 1.,
                    card.y + 1.,
                    card.w - 2.,
                    card.h - 2.,
                    2.,
                    LILAC,
                );
            }
            if ui.hit(card) {
                self.candidate = index;
                self.revision += 1;
            }
        }
        let pager = rect.bottom() - if landscape { 84. } else { 103. };
        if icon_button(
            ui,
            Glyph::Back,
            Rect::new(
                content.x + 16.,
                pager,
                44.,
                if landscape { 30. } else { 40. },
            ),
            false,
        ) {
            self.style_page = (self.style_page + self.style.count().div_ceil(page_size) - 1)
                % self.style.count().div_ceil(page_size);
            self.revision += 1;
        }
        if icon_button(
            ui,
            Glyph::Next,
            Rect::new(
                content.right() - 60.,
                pager,
                44.,
                if landscape { 30. } else { 40. },
            ),
            false,
        ) {
            self.style_page = (self.style_page + 1) % self.style.count().div_ceil(page_size);
            self.revision += 1;
        }
        ui.centered(
            &format!(
                "{} / {}",
                self.style_page + 1,
                self.style.count().div_ceil(page_size)
            ),
            Rect::new(
                content.x + 60.,
                pager,
                content.w - 120.,
                if landscape { 30. } else { 40. },
            ),
            11.,
            if ui.theme.saver { WHITE } else { INK },
            true,
        );
        let owned = self.pet.owns(self.style, self.candidate);
        let action = if self.pet.selected(self.style) == self.candidate {
            "Equipped".into()
        } else if owned {
            "Equip".into()
        } else {
            format!("Buy & equip · {} coins", self.style.price(self.candidate))
        };
        if ui.button(
            &action,
            Rect::new(content.x + 16., rect.bottom() - 52., content.w - 32., 40.),
            true,
        ) {
            let result = self.pet.customize(self.style, self.candidate);
            return self.respond(result, "Looking lovely!", now);
        }
        None
    }
    fn games_menu(&mut self, ui: &mut Ui, now: f64) -> Option<Pulse> {
        let rect = self.panel(ui, "Let's play")?;
        let cols = if rect.w > 600. || rect.w > rect.h * 1.3 {
            3
        } else {
            2
        };
        let size = if rect.h < 520. {
            if cols == 3 { 6 } else { 4 }
        } else {
            8
        };
        let pages = Kind::ALL.len().div_ceil(size);
        self.game_page %= pages;
        let rows = size.div_ceil(cols);
        let cw = (rect.w - 40. - 8. * (cols - 1) as f32) / cols as f32;
        let ch = (rect.h - 88. - if pages > 1 { 52. } else { 0. } - 8. * (rows - 1) as f32)
            / rows as f32;
        for (slot, kind) in Kind::ALL
            .into_iter()
            .skip(self.game_page * size)
            .take(size)
            .enumerate()
        {
            let i = kind.index();
            let card = Rect::new(
                rect.x + 20. + (slot % cols) as f32 * (cw + 8.),
                rect.y + 68. + (slot / cols) as f32 * (ch + 8.),
                cw,
                ch,
            );
            rounded(
                card,
                18.,
                if ui.theme.saver {
                    BLACK
                } else {
                    color_u8!(242, 248, 249, 255)
                },
            );
            let scene = Rect::new(
                card.x + 10.,
                card.y + 9.,
                card.w - 20.,
                (card.h - 53.).max(30.),
            );
            match kind {
                Kind::Catch => {
                    for v in [vec2(0.25, 0.15), vec2(0.67, 0.4)] {
                        pearl(scene.point() + v * scene.size(), 8.);
                    }
                    draw_poly(
                        scene.x + scene.w * 0.8,
                        scene.y + scene.h * 0.1,
                        10,
                        9.,
                        0.,
                        INK,
                    );
                    self.art.fish(
                        scene.point() + vec2(scene.w * 0.45, scene.h * 0.68),
                        scene.h.min(scene.w) * 0.18,
                        &self.pet,
                        0.,
                    );
                }
                Kind::Pop => {
                    for (x, y, c) in [(0.25, 0.35, PINK), (0.65, 0.6, BLUE), (0.75, 0.16, LILAC)] {
                        bubble(
                            scene.point() + vec2(x * scene.w, y * scene.h),
                            scene.h.min(scene.w) * 0.18,
                            c,
                        );
                    }
                }
                Kind::Memory => {
                    for i in 0..6 {
                        let c = Rect::new(
                            scene.x + (i % 3) as f32 * scene.w / 3. + 3.,
                            scene.y + (i / 3) as f32 * scene.h / 2. + 3.,
                            scene.w / 3. - 6.,
                            scene.h / 2. - 6.,
                        );
                        rounded(
                            c,
                            7.,
                            if ui.theme.saver {
                                BLACK
                            } else {
                                color_u8!(224, 215, 240, 255)
                            },
                        );
                        if i < 2 {
                            food_icon(1, c.center(), c.h.min(c.w) * 0.28);
                        } else {
                            glyph(Glyph::Swim, c.center(), c.h.min(c.w) * 0.25);
                        }
                    }
                }
                Kind::Hop => {
                    for i in 0..3 {
                        let c = scene.point()
                            + vec2(
                                scene.w * if i % 2 == 0 { 0.27 } else { 0.73 },
                                scene.h * (0.78 - i as f32 * 0.3),
                            );
                        ellipse(c, vec2(scene.w * 0.21, 6.), MINT);
                    }
                    self.art.fish(
                        scene.point() + vec2(scene.w * 0.27, scene.h * 0.55),
                        scene.h.min(scene.w) * 0.16,
                        &self.pet,
                        0.,
                    );
                }
                Kind::Swim => {
                    rounded(
                        Rect::new(scene.x + scene.w * 0.72, scene.y, 12., scene.h * 0.35),
                        5.,
                        MINT,
                    );
                    rounded(
                        Rect::new(
                            scene.x + scene.w * 0.72,
                            scene.y + scene.h * 0.72,
                            12.,
                            scene.h * 0.28,
                        ),
                        5.,
                        MINT,
                    );
                    self.art.fish(
                        scene.point() + vec2(scene.w * 0.35, scene.h * 0.48),
                        scene.h.min(scene.w) * 0.18,
                        &self.pet,
                        0.,
                    );
                }
                Kind::Rally => {
                    for i in 0..12 {
                        rounded(
                            Rect::new(
                                scene.x + (i % 6) as f32 * scene.w / 6. + 2.,
                                scene.y + (i / 6) as f32 * 12.,
                                scene.w / 6. - 4.,
                                9.,
                            ),
                            3.,
                            [PINK, MINT, GOLD][i % 3],
                        );
                    }
                    pearl(scene.center(), 7.);
                    rounded(
                        Rect::new(scene.center().x - 22., scene.bottom() - 10., 44., 7.),
                        3.,
                        BLUE,
                    );
                }
                Kind::Dodge => {
                    for (x, y) in [(0.2, 0.2), (0.8, 0.5), (0.65, 0.08)] {
                        draw_poly(scene.x + x * scene.w, scene.y + y * scene.h, 8, 8., 0., INK);
                    }
                    pearl(scene.point() + vec2(scene.w * 0.45, scene.h * 0.35), 7.);
                    self.art.fish(
                        scene.point() + vec2(scene.w * 0.4, scene.h * 0.72),
                        scene.h.min(scene.w) * 0.14,
                        &self.pet,
                        0.,
                    );
                }
                Kind::Beats => {
                    for i in 0..3 {
                        let x = scene.x + (i as f32 + 0.5) * scene.w / 3.;
                        draw_line(x, scene.y, x, scene.bottom(), 2., LILAC);
                        bubble(
                            vec2(x, scene.y + scene.h * (0.2 + i as f32 * 0.18)),
                            9.,
                            [PINK, BLUE, MINT][i],
                        );
                        draw_circle(x, scene.bottom() - 7., 6., [PINK, BLUE, MINT][i]);
                    }
                }
            }
            ui.centered(
                kind.title(),
                Rect::new(card.x, card.bottom() - 42., card.w, 21.),
                13.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
            ui.centered(
                &format!("Best {}", self.pet.best[i]),
                Rect::new(card.x, card.bottom() - 22., card.w, 16.),
                10.,
                if ui.theme.saver { WHITE } else { INK },
                false,
            );
            if ui.hit(card) {
                if self.pet.sleeping {
                    return self.respond(Outcome::Sleeping, "", now);
                }
                self.round = Some(Round::new(kind, (miniquad::date::now() * 1000000.) as u64));
                self.last_frame = Some(now);
                self.overlay = Overlay::None;
                self.revision += 1;
                ui.reset_focus();
                return Some(Pulse::Tap);
            }
        }
        if pages > 1 {
            let y = rect.bottom() - 56.;
            if icon_button(ui, Glyph::Back, Rect::new(rect.x + 20., y, 44., 44.), false) {
                self.game_page = (self.game_page + pages - 1) % pages;
                self.revision += 1;
            }
            if icon_button(
                ui,
                Glyph::Next,
                Rect::new(rect.right() - 64., y, 44., 44.),
                false,
            ) {
                self.game_page = (self.game_page + 1) % pages;
                self.revision += 1;
            }
            ui.centered(
                &format!("{} / {}", self.game_page + 1, pages),
                Rect::new(rect.x + 74., y, rect.w - 148., 44.),
                11.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
        }
        None
    }
    fn draw_round(&mut self, ui: &mut Ui, press: Option<Vec2>, now: f64) -> Option<Pulse> {
        let viewport = Rect::new(0., 0., screen_width(), screen_height());
        self.art.room(
            viewport,
            Room::Playroom,
            self.pet.background,
            ui.theme.saver,
            false,
        );
        if !ui.theme.saver {
            draw_rectangle(
                0.,
                0.,
                screen_width(),
                screen_height(),
                Color::new(0.86, 0.96, 0.96, 0.82),
            );
        }
        let back = Rect::new(12., 12., 44., 44.);
        if icon_button(ui, Glyph::Back, back, false) {
            self.round = None;
            self.overlay = Overlay::Games;
            self.revision += 1;
            ui.reset_focus();
            return Some(Pulse::Tap);
        }
        let w = (screen_width() - 24.).min(740.);
        let x = (screen_width() - w) / 2.;
        let area = Rect::new(x, 112., w, screen_height() - 168.);
        let finished_before = self.round.as_ref().unwrap().finished;
        if finished_before
            && icon_button(
                ui,
                Glyph::Close,
                Rect::new(screen_width() - 56., 12., 44., 44.),
                false,
            )
        {
            self.round = None;
            self.overlay = Overlay::Games;
            self.revision += 1;
            ui.reset_focus();
            return Some(Pulse::Tap);
        }
        let dt = self.last_frame.replace(now).map_or(0., |last| now - last);
        let r = self.round.as_mut().unwrap();
        let active = !r.paused && !r.finished;
        if !r.finished
            && icon_button(
                ui,
                if r.paused { Glyph::Play } else { Glyph::Pause },
                Rect::new(screen_width() - 56., 12., 44., 44.),
                false,
            )
            && !r.finished
        {
            r.paused = !r.paused;
            self.last_frame = None;
            self.revision += 1;
        }
        ui.centered(
            r.kind.title(),
            Rect::new(66., 12., screen_width() - 132., 44.),
            17.,
            if ui.theme.saver { WHITE } else { INK },
            true,
        );
        let score = Rect::new(x, 66., w, 34.);
        glass(ui, score, 15.);
        ui.label(
            &format!("{} pts · Lv {}", r.score, r.level()),
            x + 14.,
            89.,
            14.,
            if ui.theme.saver { WHITE } else { INK },
        );
        ui.centered(
            &format!("{}s", r.remaining().ceil() as u32),
            Rect::new(x + w - 66., 66., 60., 34.),
            14.,
            LILAC,
            true,
        );
        if matches!(
            r.kind,
            Kind::Catch
                | Kind::Pop
                | Kind::Hop
                | Kind::Swim
                | Kind::Rally
                | Kind::Dodge
                | Kind::Beats
        ) {
            for i in 0..r.lives {
                heart(vec2(x + w * 0.53 + f32::from(i) * 18., 83.), 7., PINK);
            }
        }
        rounded(
            area,
            25.,
            if ui.theme.saver {
                BLACK
            } else {
                color_u8!(230, 244, 248, 255)
            },
        );
        if !ui.theme.saver {
            for i in 0..8 {
                let px = area.x + area.w * (i as f32 + 0.5) / 8.;
                let py = area.y
                    + 20.
                    + ((r.elapsed as f32 * 18. + i as f32 * 57.) % (area.h - 40.).max(1.));
                bubble(
                    vec2(px, py),
                    3. + (i % 3) as f32,
                    color_u8!(200, 225, 231, 255),
                );
            }
            rounded(
                Rect::new(area.x + 2., area.bottom() - 14., area.w - 4., 12.),
                6.,
                color_u8!(213, 227, 213, 255),
            );
        }
        if r.combo >= 3 {
            ui.centered(
                &format!("{} streak", r.combo),
                Rect::new(area.x, area.y + 8., area.w, 20.),
                11.,
                LILAC,
                true,
            );
        }
        let ts = touches();
        let pointer = ts
            .iter()
            .find(|t| t.phase != TouchPhase::Cancelled)
            .map(|t| touch_point(t.position, screen_dpi_scale()))
            .or(press)
            .or_else(|| {
                is_mouse_button_down(MouseButton::Left).then(|| {
                    let (x, y) = mouse_position();
                    vec2(x, y)
                })
            });
        let mut pulse = None;
        if active && !r.paused {
            match r.kind {
                Kind::Catch | Kind::Rally | Kind::Dodge => {
                    if let Some(p) = pointer
                        && area.contains(p)
                    {
                        r.steer((p.x - area.x) / area.w);
                    }
                    if !ui.keyboard_focus {
                        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                            r.steer(r.player - dt as f32 * 0.8);
                        }
                        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                            r.steer(r.player + dt as f32 * 0.8);
                        }
                    }
                }
                Kind::Pop => {
                    if !ui.keyboard_focus {
                        let step = dt as f32 * 0.6;
                        for (key, delta) in [
                            (KeyCode::Left, (-step, 0.)),
                            (KeyCode::Right, (step, 0.)),
                            (KeyCode::Up, (0., -step)),
                            (KeyCode::Down, (0., step)),
                        ] {
                            if is_key_down(key) {
                                r.aim.0 = (r.aim.0 + delta.0).clamp(0., 1.);
                                r.aim.1 = (r.aim.1 + delta.1).clamp(0., 1.);
                            }
                        }
                        if is_key_pressed(KeyCode::Space) && r.pop(r.aim.0, r.aim.1) {
                            pulse = Some(Pulse::Tap);
                        }
                    }
                    if let Some(p) = press.filter(|p| area.contains(*p)) {
                        let radius = area.w.min(area.h) * 0.075;
                        let hit = r
                            .bubbles
                            .iter()
                            .find(|b| {
                                (p - (area.point() + vec2(b.x * area.w, b.y * area.h))).length()
                                    <= radius
                            })
                            .map(|b| (b.x, b.y));
                        if let Some((x, y)) = hit {
                            if r.pop(x, y) {
                                pulse = Some(Pulse::Tap);
                            }
                        } else {
                            r.pop((p.x - area.x) / area.w, (p.y - area.y) / area.h);
                        }
                    }
                }
                Kind::Beats => {
                    if let Some(p) = press.filter(|p| area.contains(*p)) {
                        let lane = (((p.x - area.x) / area.w) * 3.).floor().min(2.) as usize;
                        if r.beat(lane) {
                            pulse = Some(Pulse::Tap);
                        }
                    }
                    if !ui.keyboard_focus {
                        for (lane, key) in
                            [KeyCode::A, KeyCode::S, KeyCode::D].into_iter().enumerate()
                        {
                            if is_key_pressed(key) && r.beat(lane) {
                                pulse = Some(Pulse::Tap);
                            }
                        }
                    }
                }
                Kind::Memory => {}
                Kind::Hop => {
                    if let Some(p) = press
                        && area.contains(p)
                        && r.hop(u8::from(p.x > area.center().x))
                    {
                        pulse = Some(Pulse::Tap);
                    }
                    if !ui.keyboard_focus {
                        if is_key_pressed(KeyCode::Left) && r.hop(0) {
                            pulse = Some(Pulse::Tap);
                        }
                        if is_key_pressed(KeyCode::Right) && r.hop(1) {
                            pulse = Some(Pulse::Tap);
                        }
                    }
                }
                Kind::Swim => {
                    if (press.is_some_and(|p| area.contains(p))
                        || (!ui.keyboard_focus && is_key_pressed(KeyCode::Space)))
                        && r.flap()
                    {
                        pulse = Some(Pulse::Tap);
                    }
                }
            }
            if r.advance(dt) > 0 {
                pulse = Some(Pulse::Eat);
            }
        }
        match r.kind {
            Kind::Catch => {
                for d in &r.drops {
                    let p = area.point() + vec2(d.x * area.w, d.y * area.h);
                    if area.contains(p) {
                        if d.danger {
                            draw_poly(p.x, p.y, 10, 14., 18., INK);
                            draw_circle(p.x - 3., p.y - 2., 2., WHITE);
                            draw_circle(p.x + 4., p.y - 2., 2., WHITE);
                        } else {
                            pearl(p, 12.);
                        }
                    }
                }
                self.art.fish(
                    area.point() + vec2(r.player * area.w, area.h * 0.89),
                    26.,
                    &self.pet,
                    (now * 8.) as f32,
                );
            }
            Kind::Pop => {
                for b in &r.bubbles {
                    let p = area.point() + vec2(b.x * area.w, b.y * area.h);
                    let radius = area.w.min(area.h) * 0.075;
                    let color = if b.danger {
                        INK
                    } else if b.age > 2.8 {
                        GOLD
                    } else {
                        [BLUE, PINK, LILAC][(b.x * 10.) as usize % 3]
                    };
                    bubble(p, radius, color);
                    if b.danger {
                        draw_poly(p.x, p.y, 10, radius * 0.45, (r.elapsed * 50.) as f32, INK);
                        draw_circle(p.x - 3., p.y - 2., 2., WHITE);
                        draw_circle(p.x + 3., p.y - 2., 2., WHITE);
                    } else {
                        pearl(p, radius * 0.34);
                    }
                    draw_arc(
                        p.x,
                        p.y,
                        48,
                        radius + 4.,
                        -90.,
                        1.5,
                        360. * (1. - b.age / 3.5).clamp(0., 1.),
                        color,
                    );
                }
                if is_key_down(KeyCode::Left)
                    || is_key_down(KeyCode::Right)
                    || is_key_down(KeyCode::Up)
                    || is_key_down(KeyCode::Down)
                    || is_key_pressed(KeyCode::Space)
                {
                    let p = area.point() + vec2(r.aim.0 * area.w, r.aim.1 * area.h);
                    draw_circle_lines(p.x, p.y, 10., 1.5, INK);
                    draw_line(p.x - 15., p.y, p.x + 15., p.y, 1., INK);
                    draw_line(p.x, p.y - 15., p.x, p.y + 15., 1., INK);
                }
            }
            Kind::Memory => {
                let gap = 8.;
                let cols = if area.w > area.h * 1.3 { 6 } else { 3 };
                let rows = 12 / cols;
                let cw = (area.w - 40. - gap * (cols - 1) as f32) / cols as f32;
                let ch = (area.h - 32. - gap * (rows - 1) as f32) / rows as f32;
                for i in 0..12 {
                    let hit_card = Rect::new(
                        area.x + 20. + (i % cols) as f32 * (cw + gap),
                        area.y + 16. + (i / cols) as f32 * (ch + gap),
                        cw,
                        ch,
                    );
                    let t = ((r.elapsed - r.flipped_at[i]) / 0.26).clamp(0., 1.) as f32;
                    let flip = if ui.theme.saver {
                        1.
                    } else {
                        (t * std::f32::consts::PI).cos().abs().max(0.04)
                    };
                    let card = Rect::new(
                        hit_card.center().x - hit_card.w * flip / 2.,
                        hit_card.y,
                        hit_card.w * flip,
                        hit_card.h,
                    );
                    rounded(
                        card,
                        14.,
                        if ui.theme.saver {
                            BLACK
                        } else if r.matched[i] {
                            color_u8!(221, 240, 227, 255)
                        } else if r.shown[i] {
                            color_u8!(255, 242, 230, 255)
                        } else {
                            color_u8!(228, 217, 242, 255)
                        },
                    );
                    if ui.theme.saver {
                        draw_rectangle_lines(
                            card.x,
                            card.y,
                            card.w,
                            card.h,
                            1.5,
                            if r.matched[i] {
                                MINT
                            } else if r.shown[i] {
                                GOLD
                            } else {
                                LILAC
                            },
                        );
                    }
                    if r.shown[i] && t >= 0.5 {
                        food_icon(
                            usize::from(r.cards[i]) + 1,
                            card.center(),
                            card.w.min(card.h) * 0.27,
                        );
                    } else {
                        glyph(Glyph::Swim, card.center(), card.w.min(card.h) * 0.24);
                    }
                    if !r.paused && !r.finished && ui.hit(hit_card) && r.flip(i) {
                        self.revision += 1;
                        pulse = Some(Pulse::Tap);
                    }
                }
            }
            Kind::Hop => {
                for i in (0..6).rev() {
                    let y = area.bottom()
                        - area.h * (0.22 + i as f32 * 0.12 - (1. - r.hop_phase()) * 0.12);
                    let side = r.lanes[i];
                    let p = vec2(area.x + area.w * if side == 0 { 0.27 } else { 0.73 }, y);
                    ellipse(p + vec2(0., 5.), vec2(area.w * 0.17, area.h * 0.035), BLUE);
                    ellipse(
                        p,
                        vec2(area.w * 0.17, area.h * 0.035),
                        if i == 0 { GOLD } else { MINT },
                    );
                }
                let t = r.hop_phase();
                let px =
                    r.previous_player + (r.player - r.previous_player) * (t * t * (3. - 2. * t));
                let py = area.bottom()
                    - area.h * 0.15
                    - (t * std::f32::consts::PI).sin() * area.h * 0.15;
                ellipse(
                    vec2(area.x + px * area.w, area.bottom() - area.h * 0.085),
                    vec2(23., 6.),
                    BLUE,
                );
                self.art.fish(
                    vec2(area.x + px * area.w, py),
                    area.h.min(area.w) * 0.070,
                    &self.pet,
                    (now * 6.) as f32,
                );
                rounded(
                    Rect::new(area.x + 24., area.y + 12., area.w - 48., 5.),
                    2.,
                    if ui.theme.saver { INK } else { WHITE },
                );
                rounded(
                    Rect::new(
                        area.x + 24.,
                        area.y + 12.,
                        (area.w - 48.) * r.hop_time(),
                        5.,
                    ),
                    2.,
                    GOLD,
                );
                for side in 0..2 {
                    glyph(
                        if side == 0 { Glyph::Back } else { Glyph::Next },
                        vec2(
                            area.x + area.w * if side == 0 { 0.27 } else { 0.73 },
                            area.bottom() - 24.,
                        ),
                        14.,
                    );
                }
            }
            Kind::Swim => {
                for gate in &r.gates {
                    let px = area.x + gate.x * area.w;
                    if px < area.x || px > area.right() {
                        continue;
                    }
                    let opening = (0.20 - r.elapsed as f32 * 0.001).max(0.145) + 0.03;
                    let top = area.y + (gate.gap - opening) * area.h;
                    let bottom = area.y + (gate.gap + opening) * area.h;
                    rounded(
                        Rect::new(px - 12., area.y + 6., 24., (top - area.y - 6.).max(0.)),
                        10.,
                        MINT,
                    );
                    rounded(
                        Rect::new(px - 12., bottom, 24., (area.bottom() - bottom - 6.).max(0.)),
                        10.,
                        MINT,
                    );
                    for dy in [-12., 12.] {
                        draw_circle(px + dy * 0.4, top - 10., 8., MINT);
                        draw_circle(px + dy * 0.4, bottom + 10., 8., MINT);
                    }
                }
                self.art.fish(
                    area.point() + vec2(area.w * 0.28, area.h * r.swim_y),
                    if r.protected() { 19. } else { 22. },
                    &self.pet,
                    (now * 6.) as f32,
                );
            }
            Kind::Rally => {
                for i in 0..24 {
                    if !r.extras.bricks[i] {
                        continue;
                    }
                    let rect = Rect::new(
                        area.x + (0.035 + (i % 6) as f32 * 0.16) * area.w,
                        area.y + (0.105 + (i / 6) as f32 * 0.065) * area.h,
                        area.w * 0.13,
                        area.h * 0.05,
                    );
                    rounded(rect, 7., [PINK, MINT, GOLD, LILAC][i / 6]);
                    draw_arc(
                        rect.center().x,
                        rect.bottom(),
                        24,
                        rect.h * 0.7,
                        190.,
                        1.2,
                        160.,
                        WHITE,
                    );
                }
                let p = area.point() + vec2(r.extras.ball.0 * area.w, r.extras.ball.1 * area.h);
                pearl(p, area.w.min(area.h) * 0.026);
                let pw = r.extras.paddle_width(r.elapsed) * area.w;
                rounded(
                    Rect::new(
                        area.x + r.player * area.w - pw / 2.,
                        area.y + area.h * 0.85,
                        pw,
                        10.,
                    ),
                    5.,
                    BLUE,
                );
                self.art.fish(
                    area.point() + vec2(r.player * area.w, area.h * 0.91),
                    area.h.min(area.w) * 0.034,
                    &self.pet,
                    (now * 4.) as f32,
                );
            }
            Kind::Dodge => {
                for o in &r.extras.obstacles {
                    let p = area.point() + vec2(o.x * area.w, o.y * area.h);
                    if !area.contains(p) {
                        continue;
                    }
                    if o.good {
                        pearl(p, area.w.min(area.h) * 0.027);
                    } else {
                        draw_poly(
                            p.x,
                            p.y,
                            8,
                            area.w.min(area.h) * 0.04,
                            (r.elapsed * 45.) as f32,
                            INK,
                        );
                        draw_circle(p.x - 3., p.y - 2., 2., WHITE);
                        draw_circle(p.x + 3., p.y - 2., 2., WHITE);
                    }
                }
                self.art.fish(
                    area.point() + vec2(r.player * area.w, area.h * 0.82),
                    area.w.min(area.h) * 0.038,
                    &self.pet,
                    (now * 6.) as f32,
                );
            }
            Kind::Beats => {
                let colors = [PINK, BLUE, MINT];
                for (lane, color) in colors.into_iter().enumerate() {
                    let x = area.x + area.w * (lane as f32 + 0.5) / 3.;
                    draw_line(
                        x,
                        area.y + 12.,
                        x,
                        area.bottom() - 10.,
                        2.,
                        if ui.theme.saver {
                            INK
                        } else {
                            color_u8!(207, 225, 234, 255)
                        },
                    );
                    draw_circle_lines(
                        x,
                        area.y + area.h * 0.82,
                        area.w.min(area.h) * 0.065,
                        3.,
                        color,
                    );
                    ui.centered(
                        ["A", "S", "D"][lane],
                        Rect::new(x - 22., area.bottom() - 31., 44., 24.),
                        12.,
                        if ui.theme.saver { WHITE } else { INK },
                        true,
                    );
                }
                for note in &r.extras.notes {
                    let p = area.point()
                        + vec2(area.w * (note.lane as f32 + 0.5) / 3., note.y * area.h);
                    bubble(p, area.w.min(area.h) * 0.045, colors[note.lane]);
                    pearl(p, area.w.min(area.h) * 0.017);
                }
                if r.extras.combo >= 3 {
                    ui.centered(
                        &format!("{} streak", r.extras.combo),
                        Rect::new(area.x, area.y + 8., area.w, 21.),
                        11.,
                        LILAC,
                        true,
                    );
                }
            }
        }
        for effect in &r.effects {
            let p = area.point() + vec2(effect.x * area.w, effect.y * area.h);
            let t = effect.age / 0.65;
            for i in 0..7 {
                let a = i as f32 * std::f32::consts::TAU / 7.;
                draw_circle(
                    p.x + a.cos() * t * 36.,
                    p.y + a.sin() * t * 36. - t * 15.,
                    (1. - t) * 3.,
                    if effect.good { GOLD } else { PINK },
                );
            }
        }
        ui.centered(
            r.kind.instruction(),
            Rect::new(x, area.bottom() + 10., w, 30.),
            11.,
            if ui.theme.saver { ui.theme.muted } else { INK },
            false,
        );
        let finished = r.finished;
        if let Some((game, score)) = r.claim() {
            self.pet.reward(game, score, miniquad::date::now());
            self.save();
            self.refresh = true;
            pulse = Some(Pulse::Won);
        }
        let r = self.round.as_mut().unwrap();
        if r.paused || finished {
            draw_rectangle(
                area.x,
                area.y,
                area.w,
                area.h,
                Color::new(0.1, 0.07, 0.16, 0.35),
            );
            let box_rect = Rect::new(
                area.center().x - (w - 28.).min(320.) / 2.,
                area.center().y - 97.,
                (w - 28.).min(320.),
                194.,
            );
            glass(ui, box_rect, 25.);
            ui.centered(
                if finished { "Nice swimming!" } else { "Paused" },
                Rect::new(box_rect.x, box_rect.y + 16., box_rect.w, 30.),
                21.,
                if ui.theme.saver { WHITE } else { INK },
                true,
            );
            if finished {
                ui.centered(
                    &format!("{} points · +{} coins", r.score, 5 + r.score * 2),
                    Rect::new(box_rect.x, box_rect.y + 50., box_rect.w, 22.),
                    13.,
                    LILAC,
                    true,
                );
            }
            if ui.button(
                if finished { "Play again" } else { "Resume" },
                Rect::new(box_rect.x + 14., box_rect.y + 82., box_rect.w - 28., 44.),
                true,
            ) {
                if finished {
                    let kind = r.kind;
                    self.round = Some(Round::new(kind, (miniquad::date::now() * 1000000.) as u64));
                } else {
                    r.paused = false;
                }
                self.last_frame = Some(now);
                self.revision += 1;
                ui.reset_focus();
            }
            if ui.button(
                "Back to playroom",
                Rect::new(box_rect.x + 14., box_rect.y + 136., box_rect.w - 28., 44.),
                false,
            ) {
                self.round = None;
                self.overlay = Overlay::Games;
                self.revision += 1;
                ui.reset_focus();
            }
        }
        pulse
    }
    pub fn announcement(&self) -> String {
        if let Some(r) = &self.round {
            format!(
                "Jarcade. Fih. {}. {}. Score {}. {} seconds. Coins {}.",
                r.kind.title(),
                if r.finished {
                    "Finished"
                } else if r.paused {
                    "Paused"
                } else {
                    "Running"
                },
                r.score,
                r.remaining().ceil(),
                self.pet.coins
            )
        } else {
            let overlay = match self.overlay {
                Overlay::None => "",
                Overlay::Rooms => "Rooms",
                Overlay::Pantry => "Pantry",
                Overlay::Shop => "Food shop",
                Overlay::Wardrobe => "Wardrobe",
                Overlay::Games => "Mini-games",
                Overlay::Medicine => "Potion cabinet",
                Overlay::PotionShop => "Potion shop",
            };
            let selection = if self.overlay == Overlay::Wardrobe {
                self.style_name(self.candidate)
            } else if self.room == Room::Clinic {
                POTIONS[self.selected_potion].name
            } else {
                FOODS[self.selected_food].name
            };
            format!(
                "Jarcade. Fih. {}. Level {}. Coins {}. Food {:.0}. Joy {:.0}. Clean {:.0}. Rest {:.0}. Health {:.0}. {}. Color {}. Clothes {}. Hat {}. Background {}. {}. Preview {}. {}",
                self.room.title(),
                self.pet.level(),
                self.pet.coins,
                self.pet.food,
                self.pet.joy,
                self.pet.clean,
                self.pet.energy,
                self.pet.health,
                if self.pet.sleeping {
                    "Sleeping"
                } else {
                    "Awake"
                },
                self.pet.color,
                self.pet.clothes,
                self.pet.hat,
                self.pet.background,
                overlay,
                selection,
                self.message
            )
        }
    }
}
pub struct FihPreview {
    target: RenderTarget,
}
impl FihPreview {
    pub fn new(pet: &Fih, saver: bool) -> Self {
        let art = Art::shared();
        let target = render_target(1024, 1024);
        target.texture.set_filter(FilterMode::Linear);
        set_camera(&Camera2D {
            render_target: Some(target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0., 0., 1024., 1024.))
        });
        art.room(
            Rect::new(0., 0., 1024., 1024.),
            Room::Kitchen,
            pet.background,
            saver,
            false,
        );
        art.fish(vec2(512., 486.), 265., pet, 0.);
        set_default_camera();
        Self { target }
    }
    pub fn draw(&self, rect: Rect) {
        draw_texture_ex(
            &self.target.texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(rect.size()),
                flip_y: true,
                ..Default::default()
            },
        );
    }
}

fn active_pointer() -> Option<Vec2> {
    let ts = touches();
    if ts.iter().any(|t| t.phase == TouchPhase::Cancelled) || ts.len() > 1 {
        return None;
    }
    ts.iter()
        .find(|t| {
            matches!(
                t.phase,
                TouchPhase::Started | TouchPhase::Moved | TouchPhase::Stationary
            )
        })
        .map(|t| touch_point(t.position, screen_dpi_scale()))
        .or_else(|| {
            is_mouse_button_down(MouseButton::Left).then(|| {
                let (x, y) = mouse_position();
                vec2(x, y)
            })
        })
}
