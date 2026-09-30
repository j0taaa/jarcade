use crate::{
    layout::{Layout, touch_point},
    snake::Direction,
};
use macroquad::{
    miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, TouchPhase},
    prelude::{Rect, Vec2, vec2},
};

pub fn controls(center: Vec2) -> [(Direction, Rect); 4] {
    [
        (Direction::Up, vec2(0., -30.)),
        (Direction::Left, vec2(-62., 30.)),
        (Direction::Down, vec2(0., 30.)),
        (Direction::Right, vec2(62., 30.)),
    ]
    .map(|(direction, offset)| {
        let p = center + offset;
        (direction, Rect::new(p.x - 27., p.y - 27., 54., 54.))
    })
}

/// Read the event stream, rather than the last touch/key state of each frame.
/// A complete tap or swipe can arrive between two display refreshes.
#[derive(Default)]
pub struct SnakeInput {
    pub turns: Vec<Direction>,
    pub pointer: Option<Vec2>,
    pub interrupted: bool,
    board: Rect,
    center: Vec2,
    dpi: f32,
    active: bool,
    swipe: Option<(u64, Vec2)>,
}

impl SnakeInput {
    pub fn configure(&mut self, layout: &Layout, dpi: f32, active: bool) {
        self.board = layout.board;
        self.center = layout.controls;
        self.dpi = dpi;
        self.active = active;
        self.turns.clear();
        self.pointer = None;
        if !active {
            self.swipe = None;
        }
    }

    pub fn cancel(&mut self) {
        self.swipe = None;
        self.turns.clear();
        self.pointer = None;
    }

    fn button(&mut self, point: Vec2) {
        if let Some((direction, _)) = controls(self.center)
            .into_iter()
            .find(|(_, rect)| rect.contains(point))
        {
            self.turns.push(direction);
        }
    }
}

impl EventHandler for SnakeInput {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn window_minimized_event(&mut self) {
        self.interrupted = true;
        self.cancel();
    }
    fn window_restored_event(&mut self) {
        self.interrupted = true;
        self.cancel();
    }

    fn key_down_event(&mut self, key: KeyCode, _: KeyMods, repeat: bool) {
        if !self.active || repeat {
            return;
        }
        if let Some(direction) = match key {
            KeyCode::Up | KeyCode::W => Some(Direction::Up),
            KeyCode::Right | KeyCode::D => Some(Direction::Right),
            KeyCode::Down | KeyCode::S => Some(Direction::Down),
            KeyCode::Left | KeyCode::A => Some(Direction::Left),
            _ => None,
        } {
            self.turns.push(direction);
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            let point = touch_point(vec2(x, y), self.dpi);
            self.pointer.get_or_insert(point);
            if self.active {
                self.button(point);
            }
        }
    }

    fn touch_event(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        let point = touch_point(vec2(x, y), self.dpi);
        if phase == TouchPhase::Started {
            self.pointer.get_or_insert(point);
        }
        if !self.active {
            return;
        }
        match phase {
            TouchPhase::Started if self.swipe.is_none() => {
                self.button(point);
                if self.board.contains(point) {
                    self.swipe = Some((id, point));
                }
            }
            TouchPhase::Moved | TouchPhase::Ended => {
                if let Some((owner, origin)) = self.swipe
                    && owner == id
                {
                    let delta = point - origin;
                    // Logical pixels: short intentional swipes respond on Retina too.
                    if delta.length() >= 8. {
                        let direction = if delta.x.abs() > delta.y.abs() {
                            if delta.x > 0. {
                                Direction::Right
                            } else {
                                Direction::Left
                            }
                        } else if delta.y > 0. {
                            Direction::Down
                        } else {
                            Direction::Up
                        };
                        self.turns.push(direction);
                        self.swipe = Some((id, point));
                    }
                    if phase == TouchPhase::Ended {
                        self.swipe = None;
                    }
                }
            }
            TouchPhase::Cancelled if self.swipe.is_some_and(|(owner, _)| owner == id) => {
                self.swipe = None;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(dpi: f32) -> SnakeInput {
        let mut input = SnakeInput::default();
        input.configure(&Layout::new(390., 844.), dpi, true);
        input
    }

    #[test]
    fn keyboard_turns_keep_event_order_and_ignore_repeat() {
        let mut input = input(1.);
        input.key_down_event(KeyCode::Left, KeyMods::default(), false);
        input.key_down_event(KeyCode::Up, KeyMods::default(), false);
        input.key_down_event(KeyCode::Left, KeyMods::default(), true);
        assert_eq!(input.turns, [Direction::Left, Direction::Up]);
    }

    #[test]
    fn complete_swipe_between_frames_is_not_lost_at_any_dpi() {
        for dpi in [1., 2., 3., 4.] {
            let mut input = input(dpi);
            input.touch_event(TouchPhase::Started, 1, 100. * dpi, 200. * dpi);
            input.touch_event(TouchPhase::Moved, 1, 100. * dpi, 191. * dpi);
            input.touch_event(TouchPhase::Ended, 1, 91. * dpi, 191. * dpi);
            assert_eq!(input.turns, [Direction::Up, Direction::Left]);
            assert!(input.swipe.is_none());
        }
    }

    #[test]
    fn short_taps_hit_buttons_before_the_tick() {
        let mut input = input(3.);
        let p = controls(input.center)[0].1.center() * 3.;
        input.touch_event(TouchPhase::Started, 1, p.x, p.y);
        input.touch_event(TouchPhase::Ended, 1, p.x, p.y);
        assert_eq!(input.turns, [Direction::Up]);
    }

    #[test]
    fn menu_presses_survive_release_before_the_next_frame() {
        let mut input = input(3.);
        input.configure(&Layout::new(390., 844.), 3., false);
        input.touch_event(TouchPhase::Started, 1, 285., 975.);
        input.touch_event(TouchPhase::Ended, 1, 285., 975.);
        assert_eq!(input.pointer, Some(vec2(95., 325.)));
        assert!(input.turns.is_empty());
        input.configure(&Layout::new(390., 844.), 3., false);
        assert!(input.pointer.is_none());
    }

    #[test]
    fn jitter_other_fingers_and_inactive_screens_do_not_steer() {
        let mut input = input(1.);
        input.touch_event(TouchPhase::Started, 1, 100., 200.);
        input.touch_event(TouchPhase::Moved, 1, 103., 203.);
        input.touch_event(TouchPhase::Moved, 2, 130., 200.);
        assert!(input.turns.is_empty());
        input.cancel();
        input.configure(&Layout::new(390., 844.), 1., false);
        input.key_down_event(KeyCode::Up, KeyMods::default(), false);
        input.touch_event(TouchPhase::Ended, 1, 100., 150.);
        assert!(input.turns.is_empty());
    }
}
